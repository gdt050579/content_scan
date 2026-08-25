use std::marker::PhantomData;

use crate::object::ArenaIndex;
use crate::BufferArena;
use crate::ContentType;
use crate::ExtractRequestBuilder;
use crate::ExtractionRequest;
use crate::FindigsIterator;
use crate::FindingMetadata;
use crate::InternalFinding;
use crate::NoMetadata;
use crate::Object;
use crate::ScanObserver;
use varmap::{VarMap, VarMapPool};

/// Mutable state shared with plugins during a scan.
///
/// A single `Context` is threaded through every
/// [`ContentAnalyzer::analyze`](crate::ContentAnalyzer::analyze) call
/// so plugins can record results, emit findings, and queue extra
/// extraction:
///
/// - **[`global`](Self::global)** – lives for the entire scan and can
///   be inspected via [`ScanResult::global`] after the scan finishes.
/// - **[`local`](Self::local)** – attached to the currently scanned
///   content object. Retrievable per object from the result tree via
///   [`ScanResult::local`].
/// - **[`add_finding`](Self::add_finding)** – records a
///   [`Finding`](crate::Finding) on the current object. Iterate them
///   after the scan with [`ScanResult::findings`].
/// - **[`request_extract`](Self::request_extract)** – queues a pass
///   that runs extractors registered for another [`ContentType`] on a
///   region of the current object. Each `create_session` then receives
///   an [`ExtractionContext`](crate::ExtractionContext) with that
///   request's offset, length, and params.
///
/// The `M` type parameter is the [`FindingMetadata`](crate::FindingMetadata)
/// stored on each finding. It defaults to [`NoMetadata`](crate::NoMetadata).
///
/// `Context` is owned by the [`Scanner`](crate::Scanner) and cleared
/// automatically at the start of every scan; plugins never construct
/// one themselves.
pub struct Context<T: ContentType, M: FindingMetadata = NoMetadata> {
    pub(crate) global: VarMap,
    pub(crate) objects: Vec<Object>,
    pub(crate) path_arena: BufferArena,
    pub(crate) varmap_pool: VarMapPool,
    pub(crate) local_varmap_handle: Option<varmap::PoolHandle>,
    pub(crate) current_object_index: Option<u32>,
    pub(crate) extraction_requests_stack: Vec<ExtractionRequest<T>>,
    pub(crate) findings: Vec<InternalFinding<M>>,
    pub(crate) observer: Option<Box<dyn ScanObserver<T, M>>>,
    pub(crate) store_findings: bool,
}
impl<T: ContentType, M: FindingMetadata> Context<T, M> {
    pub(crate) fn new(observer: Option<Box<dyn ScanObserver<T, M>>>, store_findings: bool) -> Self {
        Self {
            global: VarMap::new(),
            objects: Vec::with_capacity(16),
            path_arena: BufferArena::new(),
            varmap_pool: VarMapPool::new(),
            local_varmap_handle: None,
            current_object_index: None,
            extraction_requests_stack: Vec::with_capacity(16),
            findings: Vec::with_capacity(16),
            observer,
            store_findings,
        }
    }
    pub(crate) fn clear(&mut self) {
        self.global.clear();
        self.objects.clear();
        self.path_arena.clear();
        self.varmap_pool.clear(varmap::ClearStrategy::KeepSmallestN(128));
        self.local_varmap_handle = None;
        self.current_object_index = None;
        self.extraction_requests_stack.clear();
        self.findings.clear();
    }

    /// Returns the [`VarMap`] shared across the entire scan.
    ///
    /// Use this to accumulate scan-wide statistics or findings (e.g.
    /// "total number of matches", "sum of extracted numbers"). After
    /// the scan finishes, the same values are available through
    /// [`ScanResult::global`].
    #[inline(always)]
    pub fn global(&mut self) -> &mut VarMap {
        &mut self.global
    }

    /// Queues an extraction of `content_type` from the current object.
    ///
    /// Returns a builder. Chain [`ExtractRequestBuilder::at`],
    /// [`ExtractRequestBuilder::len`], and
    /// [`ExtractRequestBuilder::param`] as needed, then call
    /// [`ExtractRequestBuilder::emit`] to commit. Dropping the builder
    /// without `emit` is a no-op (any reserved param map is returned
    /// to the pool).
    ///
    /// After this object's analyzers finish, the scanner first runs
    /// extractors registered for the object's own identified type,
    /// then runs extractors registered for `content_type` on the
    /// **same** parent, using the requested region and params. The
    /// current object does not need to have been identified as
    /// `content_type`.
    ///
    /// Several requests (of the same or different types) may be
    /// emitted from one analyzer; they run in emission order. The
    /// queue is cleared at the start of every object's scan, including
    /// nested children.
    pub fn request_extract(&mut self, content_type: T) -> ExtractRequestBuilder<'_, T, M> {
        ExtractRequestBuilder::new(self, content_type)
    }

    /// Returns the [`VarMap`] attached to the currently scanned
    /// content object.
    ///
    /// Each object gets its own local map, drawn from a pool so
    /// repeated scans do not reallocate. Values stored here can be
    /// retrieved later, per object, via [`ScanResult::local`].
    ///
    /// The map is created lazily on the first call for the current
    /// object; if a plugin never touches the local map, no memory is
    /// spent for that object.
    pub fn local(&mut self) -> &mut VarMap {
        if self.local_varmap_handle.is_none() {
            let handle = self.varmap_pool.allocate();
            self.local_varmap_handle = Some(handle);
            if let Some(idx) = self.current_object_index {
                self.objects[idx as usize].varmap_handle = Some(handle);
            }
        }
        self.varmap_pool.get_mut(self.local_varmap_handle.unwrap()).unwrap()
    }

    /// Returns the number of content objects processed so far in the
    /// current scan.
    ///
    /// This is useful for progress reporting or for imposing custom
    /// budgets from within a [`ContentAnalyzer`](crate::ContentAnalyzer)
    /// (e.g. return
    /// [`NextAction::Exit`](crate::NextAction::Exit) once a limit is
    /// exceeded).
    #[inline(always)]
    pub fn objects_scanned(&self) -> u32 {
        self.objects.len() as u32
    }

    /// Records a finding on the currently scanned object.
    ///
    /// The `finding` text is interned (hash digest, message, match
    /// string, …). `source` is an optional label for who produced it
    /// (rule name, plugin id). `metadata` is optional typed extra
    /// data of type `M`; pass `None` when using
    /// [`NoMetadata`](crate::NoMetadata).
    ///
    /// The finding is attached to the current object, so
    /// [`Finding::path`](crate::Finding::path) and
    /// [`Finding::content_type`](crate::Finding::content_type) later
    /// report that object's identity. Calling this when no object is
    /// current (outside an analyzer) is a no-op.
    ///
    /// After the scan, iterate findings with
    /// [`ScanResult::findings`] in the order they were recorded.
    ///
    /// ```ignore
    /// context.add_finding("deadbeef", Some("ComputeHash"), None);
    /// context.add_finding("embedded zip", None, Some(Severity::Info));
    /// ```
    pub fn add_finding(&mut self, finding: &str, source: Option<&str>, metadata: Option<M>) {
        if let Some(objindex) = self.current_object_index {
            let source_index = if let Some(source) = source {
                self.path_arena.alloc(source.as_bytes())
            } else {
                ArenaIndex::INVALID
            };

            if let Some(observer) = self.observer.as_mut() {
                let path = unsafe { std::str::from_utf8_unchecked(self.path_arena.get(self.objects[objindex as usize].path).unwrap_or_default()) };
                observer.on_finding(path, finding, source, metadata.as_ref());
            }
            if self.store_findings {
                self.findings.push(InternalFinding {
                    objindex,
                    finding: self.path_arena.alloc(finding.as_bytes()),
                    source: source_index,
                    metadata,
                });
            }
        }
    }
}

/// Opaque handle to a content object inside a [`ScanResult`].
///
/// Handles are obtained from [`ScanResult::root`] and the
/// navigation methods ([`parent`](ScanResult::parent),
/// [`child`](ScanResult::child),
/// [`next_sibling`](ScanResult::next_sibling)), and are used to look
/// up per-object data such as its
/// [`path`](ScanResult::path), [`content_type`](ScanResult::content_type),
/// or [`local`](ScanResult::local) [`VarMap`].
///
/// A handle is only valid for the [`ScanResult`] it came from and only
/// until the underlying [`Scanner`](crate::Scanner) is reused for
/// another scan.
#[derive(Copy, Clone, Debug)]
pub struct ScanContentHandle {
    pub(crate) index: u32,
}

/// The outcome of a [`Scanner::scan`](crate::Scanner::scan) call.
///
/// A `ScanResult` gives read-only access to:
///
/// - the [`global`](Self::global) [`VarMap`] filled by analyzers,
/// - the tree of content objects visited during the scan
///   ([`root`](Self::root), [`parent`](Self::parent),
///   [`child`](Self::child), [`next_sibling`](Self::next_sibling)),
/// - per-object information ([`path`](Self::path),
///   [`content_type`](Self::content_type),
///   [`local`](Self::local)),
/// - the [`findings`](Self::findings) recorded by analyzers via
///   [`Context::add_finding`].
///
/// The `M` type parameter is the [`FindingMetadata`](crate::FindingMetadata)
/// stored on each finding. It defaults to [`NoMetadata`](crate::NoMetadata).
///
/// The result borrows immutably from the scanner. Copy anything you
/// need to keep out of the result before starting another scan.
pub struct ScanResult<'a, T: ContentType, M: FindingMetadata = NoMetadata> {
    pub(crate) context: &'a Context<T, M>,
    _extra: PhantomData<(T, M)>,
}
impl<'a, T: ContentType, M: FindingMetadata> ScanResult<'a, T, M> {
    pub(crate) fn new(context: &'a Context<T, M>) -> Self {
        Self {
            context,
            _extra: PhantomData,
        }
    }

    /// Returns the [`VarMap`] populated by analyzers during the scan.
    ///
    /// This is the read-only counterpart of [`Context::global`].
    pub fn global(&self) -> &VarMap {
        &self.context.global
    }

    /// Returns the total number of content objects visited by the
    /// scan (including the root).
    pub fn objects_scanned(&self) -> u32 {
        self.context.objects.len() as u32
    }

    /// Returns a handle to the root content object.
    ///
    /// Returns `None` if the scan visited no objects (for instance
    /// because the top-level content was rejected by the
    /// [`Filter`](crate::Filter)).
    pub fn root(&self) -> Option<ScanContentHandle> {
        if self.context.objects.is_empty() {
            None
        } else {
            Some(ScanContentHandle { index: 0 })
        }
    }

    /// Returns the parent of `handle`, if any.
    ///
    /// Returns `None` for the root or for an invalid/detached handle.
    pub fn parent(&self, handle: ScanContentHandle) -> Option<ScanContentHandle> {
        let object = self.context.objects.get(handle.index as usize)?;
        if object.parent_index as usize >= self.context.objects.len() {
            None
        } else {
            Some(ScanContentHandle { index: object.parent_index })
        }
    }

    /// Returns the next sibling of `handle` (same parent, extracted
    /// after `handle`), if any.
    ///
    /// Combined with [`child`](Self::child), this lets you walk all
    /// children of a given object:
    ///
    /// ```ignore
    /// let mut c = res.child(parent);
    /// while let Some(h) = c {
    ///     // ...visit h...
    ///     c = res.next_sibling(h);
    /// }
    /// ```
    pub fn next_sibling(&self, handle: ScanContentHandle) -> Option<ScanContentHandle> {
        let object = self.context.objects.get(handle.index as usize)?;
        if object.next_sibling_index as usize >= self.context.objects.len() {
            None
        } else {
            Some(ScanContentHandle {
                index: object.next_sibling_index,
            })
        }
    }

    /// Returns a handle to the first child of `handle`, if any.
    ///
    /// Iterate the remaining children with
    /// [`next_sibling`](Self::next_sibling).
    pub fn child(&self, handle: ScanContentHandle) -> Option<ScanContentHandle> {
        let object = self.context.objects.get(handle.index as usize)?;
        if object.first_child_index as usize >= self.context.objects.len() {
            None
        } else {
            Some(ScanContentHandle {
                index: object.first_child_index,
            })
        }
    }

    /// Returns the per-object [`VarMap`] populated by analyzers.
    ///
    /// Returns `None` if the object never touched
    /// [`Context::local`] (its map was never allocated) or if the
    /// handle is invalid.
    pub fn local(&self, handle: ScanContentHandle) -> Option<&VarMap> {
        let object = self.context.objects.get(handle.index as usize)?;
        if let Some(varmap_handle) = object.varmap_handle {
            self.context.varmap_pool.get(varmap_handle)
        } else {
            None
        }
    }

    /// Returns the path stored for the object referenced by `handle`.
    ///
    /// This is the interned printable view of
    /// [`Content::path`](crate::Content::path) at the moment the object
    /// was scanned (see [`ContentPath::as_printable_string`](crate::ContentPath::as_printable_string)).
    /// For a live content object, prefer
    /// [`ContentPath::as_printable_string`](crate::ContentPath::as_printable_string)
    /// (display) or [`ContentPath::as_path`](crate::ContentPath::as_path)
    /// (filesystem). Returns `None` for an invalid handle.
    pub fn path(&self, handle: ScanContentHandle) -> Option<&str> {
        let object = self.context.objects.get(handle.index as usize)?;
        let path = self.context.path_arena.get(object.path)?;
        std::str::from_utf8(path).ok()
    }

    /// Returns the identified [`ContentType`] of `handle`, if any.
    ///
    /// Returns `None` when the scanner was unable to identify the
    /// object's type (no matching identifier or all identifiers
    /// rejected it), or for an invalid handle.
    pub fn content_type(&self, handle: ScanContentHandle) -> Option<T> {
        let object = self.context.objects.get(handle.index as usize)?;
        T::from_u16(object.type_id)
    }

    /// Iterates every [`Finding`](crate::Finding) recorded during the
    /// scan, in emission order.
    ///
    /// Analyzers add findings with [`Context::add_finding`]. Each
    /// finding is linked to the object that was current at the time,
    /// so [`Finding::path`](crate::Finding::path) / [`Finding::content_type`](crate::Finding::content_type)
    /// resolve against this result's object tree.
    ///
    /// ```ignore
    /// for f in res.findings() {
    ///     println!("{}  {}", f.finding(), f.path().unwrap_or("?"));
    /// }
    /// ```
    pub fn findings(&self) -> FindigsIterator<'a, T, M> {
        FindigsIterator::new(self.context)
    }
}
