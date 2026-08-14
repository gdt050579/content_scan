use std::marker::PhantomData;

use crate::BufferArena;
use crate::ContentType;
use crate::Object;
use varmap::VarMap;

/// Mutable state shared with plugins during a scan.
///
/// A single `Context` is threaded through every
/// [`ContentAnalyzer::analyze`](crate::ContentAnalyzer::analyze) call
/// so plugins can record findings in three different scopes:
///
/// - **[`global`](Self::global)** – lives for the entire scan and can
///   be inspected via [`ScanResult::global`] after the scan finishes.
/// - **[`extract`](Self::extract)** – lives for the duration of a
///   single extraction pass. Handed to
///   [`ContentExtractor::acquire`](crate::ContentExtractor::acquire) so
///   an extractor can hold per-run state without allocating.
/// - **[`local`](Self::local)** – attached to the currently scanned
///   content object. Retrievable per object from the result tree via
///   [`ScanResult::local`].
///
/// `Context` is owned by the [`Scanner`](crate::Scanner) and cleared
/// automatically at the start of every scan; plugins never construct
/// one themselves.
pub struct Context {
    pub(crate) global: VarMap,
    pub(crate) extract: VarMap,
    pub(crate) objects: Vec<Object>,
    pub(crate) path_arena: BufferArena,
    pub(crate) varmap_pool: Vec<VarMap>,
    pub(crate) used_local_varmaps: u32,
    pub(crate) local_varmaps_index: u32,
}
impl Context {
    pub(crate) fn new() -> Self {
        Self {
            global: VarMap::new(),
            extract: VarMap::new(),
            objects: Vec::with_capacity(16),
            path_arena: BufferArena::new(),
            varmap_pool: Vec::with_capacity(16),
            used_local_varmaps: 0,
            local_varmaps_index: Object::INVALID_INDEX,
        }
    }
    pub(crate) fn clear(&mut self) {
        self.global.clear();
        self.extract.clear();
        self.objects.clear();
        self.path_arena.clear();
        self.varmap_pool.truncate(128);
        for varmap in self.varmap_pool.iter_mut() {
            varmap.clear();
        }
        self.used_local_varmaps = 0;
        self.local_varmaps_index = Object::INVALID_INDEX;
    }
    pub(crate) fn clear_extract(&mut self) {
        self.extract.clear();
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

    /// Returns the [`VarMap`] scoped to the current extraction.
    ///
    /// This map is cleared each time the scanner moves to a new
    /// content object and is passed to
    /// [`ContentExtractor::acquire`](crate::ContentExtractor::acquire).
    /// Analyzers may also read/write it while an extraction is in
    /// progress.
    #[inline(always)]
    pub fn extract(&mut self) -> &mut VarMap {
        &mut self.extract
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
        if self.local_varmaps_index == Object::INVALID_INDEX {
            if self.used_local_varmaps >= self.varmap_pool.len() as u32 {
                self.varmap_pool.push(VarMap::new());
                self.used_local_varmaps = self.varmap_pool.len() as u32;
                self.local_varmaps_index = self.used_local_varmaps - 1;
            } else {
                self.local_varmaps_index = self.used_local_varmaps;
                self.used_local_varmaps += 1;
                self.varmap_pool[self.local_varmaps_index as usize].clear();
            }
            // link to the object
            if let Some(object) = self.objects.last_mut() {
                object.varmap_index = self.local_varmaps_index;
            }
        }
        &mut self.varmap_pool[self.local_varmaps_index as usize]
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
///   [`local`](Self::local)).
///
/// The result borrows immutably from the scanner. Copy anything you
/// need to keep out of the result before starting another scan.
pub struct ScanResult<'a, T: ContentType> {
    pub(crate) context: &'a Context,
    _extra: PhantomData<T>,
}
impl<'a, T: ContentType> ScanResult<'a, T> {
    pub(crate) fn new(context: &'a Context) -> Self {
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
            Some(ScanContentHandle { index: object.next_sibling_index })
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
            Some(ScanContentHandle { index: object.first_child_index })
        }
    }

    /// Returns the per-object [`VarMap`] populated by analyzers.
    ///
    /// Returns `None` if the object never touched
    /// [`Context::local`] (its map was never allocated) or if the
    /// handle is invalid.
    pub fn local(&self, handle: ScanContentHandle) -> Option<&VarMap> {
        let object = self.context.objects.get(handle.index as usize)?;
        if object.varmap_index as usize >= self.context.varmap_pool.len() {
            None
        } else {
            self.context.varmap_pool.get(object.varmap_index as usize)
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
}
