use crate::ExtractionHandle;

use super::{Content, ContentType, Context};
use varmap::VarMap;

/// Return value used by analyzers and extractors to steer the scan.
///
/// Every analyzer and extractor invocation returns a `NextAction` that
/// tells the scanner what to do next. This lets a single plugin either
/// short-circuit the current object or abort the entire scan.
pub enum NextAction {
    /// Continue with the next plugin (or the next extracted entry).
    ///
    /// This is the normal / neutral outcome.
    Continue,
    /// Stop processing the current content object.
    ///
    /// The scanner drops any remaining analyzers/extractors for the
    /// current object but keeps scanning siblings and subsequent
    /// content.
    Skip,
    /// Abort the entire scan immediately.
    ///
    /// No further plugins run and control returns to
    /// [`Scanner::scan`](crate::Scanner::scan) as soon as the current
    /// call stack unwinds.
    Exit,
}

/// Metadata for a single entry produced by a [`ContentExtractor`].
///
/// An extractor advertises the next item it is about to extract via
/// this struct. The scanner uses it to apply
/// [`Filter`](crate::Filter) rules cheaply (path + size) before asking
/// the extractor to materialize the actual [`Content`].
#[derive(Default)]
pub struct Entry {
    /// Virtual path of the entry (for example, the name inside an
    /// archive).
    pub path: String,
    /// Size of the entry in bytes, when known.
    pub size: u64,
    /// When `true`, the scanner does not test this entry against the
    /// active [`Filter`](crate::Filter).
    ///
    /// Use it for entries that are containers rather than payloads and
    /// would never pass the filter on their own — for example
    /// subfolders enumerated by
    /// [`FolderExtractor`](crate::FolderExtractor) while the filter
    /// only allows a set of file extensions.
    pub skip_from_filtering: bool,
}
impl Entry {
    pub fn update(&mut self, path: &str, size: u64, skip_from_filtering: bool) {
        self.path.clear();
        self.path.push_str(path);
        self.size = size;
        self.skip_from_filtering = skip_from_filtering;
    }
}

/// A plugin that inspects a piece of content and records information.
///
/// Analyzers are the "read-only" half of the framework: they observe
/// content and write findings into the [`Context`] (global, per-scan,
/// or per-object [`VarMap`]s). They do not produce new content — for
/// that, use a [`ContentExtractor`].
///
/// Analyzers can be registered per [`ContentType`] via
/// [`ScannerBuilder::add_analyzer`](crate::ScannerBuilder::add_analyzer)
/// or as generic (all types) plugins via
/// [`ScannerBuilder::add_generic_analyzer`](crate::ScannerBuilder::add_generic_analyzer).
pub trait ContentAnalyzer<T: ContentType> {
    /// Analyzes `content` and reports findings through `context`.
    ///
    /// The returned [`NextAction`] controls whether further analyzers
    /// (and extractors) run for this object.
    fn analyze(&mut self, content: &mut dyn Content<T>, context: &mut Context) -> NextAction;
}

/// A plugin that produces child content items from a parent.
///
/// Extractors are the "recursion" half of the framework. Typical
/// examples: an archive extractor emitting each stored file, an
/// executable extractor emitting resource sections, a text extractor
/// emitting embedded numeric tokens, and so on.
///
/// The scanner drives an extractor as a short session keyed by an
/// opaque [`ExtractionHandle`]:
///
/// 1. [`acquire`](Self::acquire) is called once per parent content. If
///    it returns `None`, this extractor is skipped for the parent.
///    Otherwise the returned handle identifies the session and is
///    passed to every subsequent call.
/// 2. [`advance`](Self::advance) is called repeatedly with that
///    handle. Each call returns metadata for the next available
///    entry, or `None` when the extractor is exhausted.
/// 3. For each accepted entry, [`extract`](Self::extract) is called
///    (with the same handle) to materialize a [`Content`] object which
///    is then scanned recursively (subject to the configured max
///    depth).
/// 4. [`release`](Self::release) is always called exactly once for a
///    successfully acquired handle, whether the stream ended
///    normally, a filter skipped every entry, or the scan aborted
///    early with [`NextAction::Skip`] / [`NextAction::Exit`].
///
/// The handle lets a single extractor instance keep per-session state
/// (cursor, buffers, open archive handles, …) even when extractions
/// nest or interleave. Handles are minted by an
/// [`ExtractionPool`](crate::ExtractionPool), which owns the state
/// behind them: call
/// [`acquire_slot`](crate::ExtractionPool::acquire_slot) in `acquire`,
/// [`get`](crate::ExtractionPool::get) /
/// [`get_mut`](crate::ExtractionPool::get_mut) in `advance` and
/// `extract`, and
/// [`release_slot`](crate::ExtractionPool::release_slot) in `release`.
///
/// Extractors can be registered per [`ContentType`] via
/// [`ScannerBuilder::add_extractor`](crate::ScannerBuilder::add_extractor)
/// or as generic (all types) plugins via
/// [`ScannerBuilder::add_generic_extractor`](crate::ScannerBuilder::add_generic_extractor).
pub trait ContentExtractor<T: ContentType> {
    /// Begins an extraction session over `content`.
    ///
    /// The `extract_context` [`VarMap`] is scoped to the current
    /// parent object and can be used to stash per-extraction state
    /// alongside plugin-owned fields keyed by the returned handle.
    /// The handle itself should come from an
    /// [`ExtractionPool`](crate::ExtractionPool).
    ///
    /// Return `Some(handle)` to proceed with
    /// [`advance`](Self::advance) / [`extract`](Self::extract), or
    /// `None` to skip this extractor entirely for the current parent.
    /// Every successful acquire is paired with a later
    /// [`release`](Self::release) on the same handle.
    fn acquire(&mut self, content: &mut dyn Content<T>, extract_context: &mut VarMap) -> Option<ExtractionHandle>;

    /// Advances the session identified by `handle` to the next entry.
    ///
    /// Returns `Some(&Entry)` describing the upcoming entry (path and
    /// size), or `None` when there are no more entries to extract.
    /// The scanner may consult the entry's path and size against the
    /// active [`Filter`](crate::Filter) and skip the following
    /// [`extract`](Self::extract) call accordingly.
    fn advance(&mut self, handle: ExtractionHandle, content: &mut dyn Content<T>) -> Option<&Entry>;

    /// Materializes the [`Content`] for the entry most recently
    /// announced by [`advance`](Self::advance) on `handle`.
    ///
    /// Returning `None` skips the current entry without aborting the
    /// enumeration (the scanner will still call `advance` again to
    /// look for the following entry).
    fn extract(&mut self, handle: ExtractionHandle, content: &mut dyn Content<T>) -> Option<Box<dyn Content<T>>>;

    /// Ends the extraction session identified by `handle`.
    ///
    /// Called exactly once after a successful
    /// [`acquire`](Self::acquire), including when the scan stops
    /// early. Use it to drop per-session resources (open files,
    /// cursors, pooled buffers, …) keyed by the handle.
    fn release(&mut self, handle: ExtractionHandle);
}

/// Fast, declarative way to identify a content type without running
/// custom code.
///
/// A [`ContentIdentifier`] can return one of these variants from
/// [`identify_method`](ContentIdentifier::identify_method) so the
/// scanner can build efficient matchers (tries, magic tables, etc.)
/// once and reuse them for every scanned object.
///
/// If none of the variants fit your identifier, return `None` from
/// `identify_method` and perform your own check inside
/// [`validate`](ContentIdentifier::validate).
pub enum IdentifyMethod {
    /// Content whose first bytes exactly match this magic sequence.
    Magic(&'static [u8]),
    /// Content whose first bytes match any of these magic sequences.
    MultipleMagic(&'static [&'static [u8]]),
    /// Content whose file extension exactly matches this string (case
    /// sensitive, without the leading dot).
    Extension(&'static str),
    /// Content whose file extension matches any of these strings.
    Extensions(&'static [&'static str]),
    /// Content whose file name exactly matches this string.
    Name(&'static str),
    /// Content whose file name matches any of these strings.
    Names(&'static [&'static str]),
}

/// A plugin that classifies content into a [`ContentType`].
///
/// Identifiers pair a fast pre-filter ([`IdentifyMethod`]) with an
/// optional custom [`validate`](Self::validate) step. The scanner
/// first uses the pre-filter to narrow down candidate types and then
/// calls `validate` to accept or reject each candidate.
///
/// Register identifiers via
/// [`ScannerBuilder::add_identifier`](crate::ScannerBuilder::add_identifier).
/// Only one identifier is allowed per [`ContentType`].
pub trait ContentIdentifier<T: ContentType> {
    /// Returns the fast pre-filter used by the scanner, if any.
    ///
    /// Return `None` to disable pre-filtering and rely solely on
    /// [`validate`](Self::validate).
    fn identify_method(&self) -> Option<IdentifyMethod>;

    /// Confirms that `content` is really of the type this identifier
    /// is registered for.
    ///
    /// Called after the pre-filter matched (or unconditionally when no
    /// pre-filter is provided). Returning `false` rejects the
    /// candidate and lets the scanner try the next possible type.
    fn validate(&self, content: &dyn Content<T>) -> bool;
}
