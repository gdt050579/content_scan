use crate::{object::ArenaIndex, ContentType, Context};

/// Typed extra data attached to a [`Finding`].
///
/// Every [`Scanner`](crate::Scanner), [`Context`](crate::Context), and
/// [`ScanResult`](crate::ScanResult) is parameterized by a metadata
/// type `M`. Analyzers that call
/// [`Context::add_finding`](crate::Context::add_finding) may attach
/// one `M` value per finding (severity, offset, rule id, …).
///
/// The default is [`NoMetadata`], which carries nothing. To use a
/// custom type, implement this marker trait and build the scanner
/// with [`ScannerBuilder::with_metadata`](crate::ScannerBuilder::with_metadata):
///
/// ```ignore
/// #[derive(Copy, Clone, Debug)]
/// enum Severity { Info, Warn, Error }
/// impl FindingMetadata for Severity {}
///
/// let mut scanner = ScannerBuilder::<MyTypes>::with_metadata::<Severity>()
///     .add_generic_analyzer(0, MyAnalyzer {})
///     .build();
/// ```
///
/// `Copy` is required so findings stay cheap to store and iterate.
pub trait FindingMetadata: Copy {}

/// Empty metadata type used when findings carry no extra data.
///
/// This is the default `M` on [`ScannerBuilder`](crate::ScannerBuilder),
/// [`Context`](crate::Context), [`ScanResult`](crate::ScanResult), and
/// [`ContentAnalyzer`](crate::ContentAnalyzer). Analyzers then pass
/// `None` as the metadata argument to
/// [`Context::add_finding`](crate::Context::add_finding).
#[derive(Copy, Clone, Debug)]
pub struct NoMetadata;
impl FindingMetadata for NoMetadata {}

pub(crate) struct InternalFinding<M: FindingMetadata> {
    pub(crate) objindex: u32,
    pub(crate) finding: ArenaIndex,
    pub(crate) source: ArenaIndex,
    pub(crate) metadata: Option<M>,
}

/// One finding recorded by an analyzer during a scan.
///
/// Findings are written with [`Context::add_finding`](crate::Context::add_finding)
/// and read after the scan via [`ScanResult::findings`](crate::ScanResult::findings).
/// Each finding belongs to the content object that was current when
/// it was recorded, and optionally carries a source label and typed
/// [`FindingMetadata`].
///
/// The finding text and source are interned into the scanner's path
/// arena, so the borrowed strings stay valid for the lifetime of the
/// [`ScanResult`](crate::ScanResult) they came from.
///
/// ```ignore
/// for f in res.findings() {
///     println!("{}  {}", f.finding(), f.path().unwrap_or("?"));
///     if let Some(src) = f.source() {
///         println!("  from {src}");
///     }
/// }
/// ```
pub struct Finding<'a, T: ContentType, M: FindingMetadata> {
    inner: &'a InternalFinding<M>,
    ctx: &'a Context<T, M>,
}

impl<'a, T: ContentType, M: FindingMetadata> Finding<'a, T, M> {
    /// Optional source label supplied when the finding was recorded.
    ///
    /// Typical values are a rule name, plugin id, or file that
    /// produced the finding. Returns `None` when
    /// [`Context::add_finding`](crate::Context::add_finding) was
    /// called with `source = None`.
    pub fn source(&self) -> Option<&'a str> {
        if self.inner.source.is_valid() {
            self.ctx
                .path_arena
                .get(self.inner.source)
                .map(|s| unsafe { std::str::from_utf8_unchecked(s) })
        } else {
            None
        }
    }

    /// The finding text itself (hash, message, match, …).
    ///
    /// This is the `finding` string passed to
    /// [`Context::add_finding`](crate::Context::add_finding). Empty
    /// only if the interned bytes were lost, which does not happen
    /// for a finding obtained from a live [`ScanResult`](crate::ScanResult).
    pub fn finding(&self) -> &'a str {
        self.ctx
            .path_arena
            .get(self.inner.finding)
            .map(|s| unsafe { std::str::from_utf8_unchecked(s) })
            .unwrap_or_default()
    }

    /// Typed metadata attached to this finding, if any.
    ///
    /// The type is the `M` the scanner was built with. Returns `None`
    /// when the analyzer passed `None` to
    /// [`Context::add_finding`](crate::Context::add_finding), or when
    /// `M` is [`NoMetadata`].
    pub fn metadata(&self) -> Option<&'a M> {
        self.inner.metadata.as_ref()
    }

    /// Identified [`ContentType`] of the object that produced this
    /// finding.
    ///
    /// Returns `None` when that object was never identified, or if
    /// the stored index is stale.
    pub fn content_type(&self) -> Option<T> {
        self.ctx.objects.get(self.inner.objindex as usize).and_then(|f| T::from_u16(f.type_id))
    }

    /// Printable path of the object that produced this finding.
    ///
    /// Same interned view as [`ScanResult::path`](crate::ScanResult::path)
    /// for that object. Returns `None` if the stored index is stale.
    pub fn path(&self) -> Option<&'a str> {
        if let Some(obj) = self.ctx.objects.get(self.inner.objindex as usize) {
            self.ctx.path_arena.get(obj.path).map(|s| unsafe { std::str::from_utf8_unchecked(s) })
        } else {
            None
        }
    }
}

/// Iterator over the findings of a [`ScanResult`](crate::ScanResult).
///
/// Produced by [`ScanResult::findings`](crate::ScanResult::findings).
/// Yields [`Finding`]s in the order they were recorded. The iterator
/// borrows the result for its lifetime; do not start another scan on
/// the same [`Scanner`](crate::Scanner) while it is still in use.
pub struct FindigsIterator<'a, T: ContentType, M: FindingMetadata> {
    id: u32,
    len: u32,
    ctx: &'a Context<T, M>,
}

impl<'a, T: ContentType, M: FindingMetadata> FindigsIterator<'a, T, M> {
    pub(crate) fn new(ctx: &'a Context<T, M>) -> Self {
        let len = ctx.findings.len() as u32;
        Self { id: 0, len, ctx }
    }
}

impl<'a, T: ContentType, M: FindingMetadata> Iterator for FindigsIterator<'a, T, M> {
    type Item = Finding<'a, T, M>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.id >= self.len {
            return None;
        }
        let innner_data = &self.ctx.findings[self.id as usize];
        self.id += 1;
        Some(Finding {
            inner: innner_data,
            ctx: self.ctx,
        })
    }
}
