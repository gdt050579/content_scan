/// Opaque token identifying one extraction session on a
/// [`ContentExtractor`](crate::ContentExtractor).
///
/// The scanner obtains a handle from
/// [`ContentExtractor::acquire`](crate::ContentExtractor::acquire) and
/// passes it back into every subsequent
/// [`advance`](crate::ContentExtractor::advance) /
/// [`extract`](crate::ContentExtractor::extract) /
/// [`release`](crate::ContentExtractor::release) call for that session.
/// This lets a single extractor instance drive several concurrent
/// (or nested) extractions by keying per-session state off the handle.
///
/// Extractors mint handles themselves via [`ExtractionHandle::new`].
/// The `index` / `uid` pair is meaningful only to the extractor that
/// created it; the scanner treats the value as opaque.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ExtractionHandle {
    pub(crate) index: u32,
    pub(crate) uid: u32,
}

impl ExtractionHandle {
    /// Creates a new handle with the given `index` and `uid`.
    ///
    /// Simple extractors that only ever run one session at a time can
    /// return a constant handle (for example `ExtractionHandle::new(0, 0)`).
    /// Extractors that multiplex sessions should pick a unique pair for
    /// each successful [`acquire`](crate::ContentExtractor::acquire).
    #[inline(always)]
    pub const fn new(index: u32, uid: u32) -> Self {
        Self { index, uid }
    }
}
