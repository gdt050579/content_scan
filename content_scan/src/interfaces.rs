use super::{Content, ContentType, Context};
use crate::ContentPath;
use crate::ExtractionContext;
use crate::OwnedContentPtr;

/// Return value used by analyzers to steer the scan.
///
/// [`ContentAnalyzer::analyze`] returns a `NextAction` that tells the
/// scanner whether to continue this object, skip the rest of it, or
/// abort the entire scan. [`ContentExtractor`] methods do not return
/// this type: they yield [`Option`] at each session step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NextAction {
    /// Continue with the next analyzer for this object, then extractors.
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
    /// Path of the entry (a filesystem path, or a synthetic address
    /// such as the name inside an archive).
    ///
    /// Extractors typically keep one `Entry` as a field and overwrite
    /// this path in place with [`ContentPath::set_from_str`] (virtual
    /// names) or [`ContentPath::set_from_os`] (real OS paths) so the
    /// allocation is reused across children.
    pub path: ContentPath,
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

/// A plugin that inspects a piece of content and records information.
///
/// Analyzers are the "read-only" half of the framework: they observe
/// content and write findings into the [`Context`]. Use
/// [`Context::local`] for per-object results and
/// [`Context::global`] for scan-wide aggregates. To run extractors
/// registered for a *different* type on a region of this object
/// (for example the byte offset of an embedded ZIP), call
/// [`Context::request_extract`]. Analyzers do not
/// produce child content themselves —
/// for that, use a [`ContentExtractor`].
///
/// Analyzers can be registered per [`ContentType`] via
/// [`ScannerBuilder::add_analyzer`](crate::ScannerBuilder::add_analyzer)
/// or as generic (all types) plugins via
/// [`ScannerBuilder::add_generic_analyzer`](crate::ScannerBuilder::add_generic_analyzer).
pub trait ContentAnalyzer<T: ContentType> {
    /// Analyzes `content` and reports findings through `context`.
    ///
    /// Use [`Context::local`] / [`Context::global`] to record results,
    /// and [`Context::request_extract`] to queue extractors of another
    /// type on a region of `content`. The returned [`NextAction`]
    /// controls whether further analyzers (and extractors) run for
    /// this object.
    fn analyze(&mut self, content: &mut dyn Content<T>, context: &mut Context<T>) -> NextAction;
}

pub trait ContentExtractor<T: ContentType> {
    fn create_session(&mut self, content: OwnedContentPtr<T>, extract_context: &ExtractionContext) -> Option<Box<dyn ExtractionSession<T>>>;
}
pub trait ExtractionSession<T: ContentType> {
    fn advance(&mut self) -> Option<&Entry>;
    fn extract(&mut self) -> Option<Box<dyn Content<T>>>;
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
///
/// Magic patterns ([`Magic`](Self::Magic),
/// [`MultipleMagic`](Self::MultipleMagic)) must be at most 16 bytes.
/// The scanner only feeds the matcher the first 16 bytes of content;
/// [`ScannerBuilder::build`](crate::ScannerBuilder::build) panics if a
/// registered magic is longer. To match bytes past that window, return
/// `None` from `identify_method` and call [`Content::read`] in
/// `validate`.
pub enum IdentifyMethod {
    /// Content whose first bytes exactly match this magic sequence.
    ///
    /// The sequence must be at most 16 bytes. Longer patterns can never
    /// match (the scanner reads only `content.read(0, 16)`) and cause
    /// [`ScannerBuilder::build`](crate::ScannerBuilder::build) to panic.
    Magic(&'static [u8]),
    /// Content whose first bytes match any of these magic sequences.
    ///
    /// Each sequence must be at most 16 bytes; see [`Magic`](Self::Magic).
    MultipleMagic(&'static [&'static [u8]]),
    /// Content whose file extension matches this string (ASCII
    /// case-insensitive, without the leading dot).
    ///
    /// `Notes.TXT` matches `Extension("txt")`; a registered pattern
    /// `"JPG"` matches `photo.jpg`.
    Extension(&'static str),
    /// Content whose file extension matches any of these strings (ASCII
    /// case-insensitive, without the leading dot).
    Extensions(&'static [&'static str]),
    /// Content whose file name (basename) matches this string (ASCII
    /// case-insensitive).
    ///
    /// `makefile` and `MAKEFILE` both match `Name("Makefile")`.
    Name(&'static str),
    /// Content whose file name (basename) matches any of these strings
    /// (ASCII case-insensitive).
    Names(&'static [&'static str]),
}

/// A plugin that classifies content into a [`ContentType`].
///
/// Identifiers pair a fast pre-filter ([`IdentifyMethod`]) with an
/// optional custom [`validate`](Self::validate) step. The scanner
/// first uses the pre-filter to narrow down candidate types and then
/// calls `validate` to accept or reject each candidate. Identifiers
/// that return `None` from [`identify_method`](Self::identify_method)
/// are tried afterwards, in registration order, via `validate` alone.
///
/// Register identifiers via
/// [`ScannerBuilder::add_identifier`](crate::ScannerBuilder::add_identifier).
/// Only one identifier is allowed per [`ContentType`].
/// [`ScannerBuilder::build`](crate::ScannerBuilder::build) panics if a
/// [`IdentifyMethod::Magic`] / [`IdentifyMethod::MultipleMagic`]
/// pattern is longer than 16 bytes.
pub trait ContentIdentifier<T: ContentType> {
    /// Returns the fast pre-filter used by the scanner, if any.
    ///
    /// Return `None` to disable pre-filtering and rely solely on
    /// [`validate`](Self::validate). Those identifiers are tried after
    /// magic, file-name, and extension matching have all been considered.
    fn identify_method(&self) -> Option<IdentifyMethod>;

    /// Confirms that `content` is really of the type this identifier
    /// is registered for.
    ///
    /// Called after the pre-filter matched (or unconditionally when no
    /// pre-filter is provided). Returning `false` rejects the
    /// candidate and lets the scanner try the next possible type.
    ///
    /// `content` is `&mut` so this method can call
    /// [`Content::read`](crate::Content::read) — for example to inspect
    /// bytes beyond the scanner's 16-byte magic window, or to implement
    /// a fully custom identifier (`identify_method` returning `None`).
    fn validate(&self, content: &mut dyn Content<T>) -> bool;
}
