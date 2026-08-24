use super::{Content, ContentType, Context};
use crate::ContentPath;
use crate::ExtractionContext;
use crate::OwnedContentPtr;
use crate::FindingMetadata;
use crate::NoMetadata;

/// Return value used by analyzers to steer the scan.
///
/// [`ContentAnalyzer::analyze`] returns a `NextAction` that tells the
/// scanner whether to continue this object, skip the rest of it, or
/// abort the entire scan. [`ContentExtractor`] and
/// [`ExtractionSession`] methods do not return this type: they yield
/// [`Option`] at each session step.
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
/// An [`ExtractionSession`] advertises the next item it is about to
/// extract via this struct. The scanner uses it to apply
/// [`Filter`](crate::Filter) rules cheaply (path + size) before asking
/// the session to materialize the actual [`Content`].
#[derive(Default)]
pub struct Entry {
    /// Path of the entry (a filesystem path, or a synthetic address
    /// such as the name inside an archive).
    ///
    /// Sessions typically keep one `Entry` as a field and overwrite
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
/// content and write results into the [`Context`]. Use
/// [`Context::local`] for per-object maps,
/// [`Context::global`] for scan-wide aggregates, and
/// [`Context::add_finding`] for a flat list of
/// [`Finding`](crate::Finding)s retrieved after the scan via
/// [`ScanResult::findings`](crate::ScanResult::findings). To run
/// extractors registered for a *different* type on a region of this
/// object (for example the byte offset of an embedded ZIP), call
/// [`Context::request_extract`]. Analyzers do not produce child
/// content themselves — for that, use a [`ContentExtractor`].
///
/// The `M` type parameter is the [`FindingMetadata`](crate::FindingMetadata)
/// attached to each finding. It defaults to
/// [`NoMetadata`](crate::NoMetadata); use
/// [`ScannerBuilder::with_metadata`](crate::ScannerBuilder::with_metadata)
/// when analyzers need a custom metadata type.
///
/// Analyzers can be registered per [`ContentType`] via
/// [`ScannerBuilder::add_analyzer`](crate::ScannerBuilder::add_analyzer)
/// or as generic (all types) plugins via
/// [`ScannerBuilder::add_generic_analyzer`](crate::ScannerBuilder::add_generic_analyzer).
///
/// Every analyzer must implement [`Dependencies`] — typically with
/// `#[derive(Dependencies)]` and a `#[Dependencies(name = "...")]`
/// attribute. In debug builds,
/// [`ScannerBuilder::build`](crate::ScannerBuilder::build) uses that
/// name (and any `requires`) to check that required analyzers are
/// registered with a strictly lower priority.
pub trait ContentAnalyzer<T: ContentType, M: FindingMetadata = NoMetadata>: Dependencies {
    /// Analyzes `content` and reports findings through `context`.
    ///
    /// Use [`Context::local`] / [`Context::global`] to record maps,
    /// [`Context::add_finding`] to emit a
    /// [`Finding`](crate::Finding), and
    /// [`Context::request_extract`] to queue extractors of another
    /// type on a region of `content`. The returned [`NextAction`]
    /// controls whether further analyzers (and extractors) run for
    /// this object.
    fn analyze(&mut self, content: &mut dyn Content<T>, context: &mut Context<T, M>) -> NextAction;
}

/// A plugin that turns a container into a stream of child [`Content`]
/// items.
///
/// Extractors are the "produce children" half of the framework. The
/// scanner calls [`create_session`](Self::create_session) once per
/// parent; the returned [`ExtractionSession`] then enumerates children
/// with [`advance`](ExtractionSession::advance) /
/// [`extract`](ExtractionSession::extract). Methods return [`Option`]
/// (or nothing, when the session is dropped) — they do **not** return
/// [`NextAction`] and cannot Skip or Exit on their own.
///
/// An extractor registered for type `T` runs in two situations:
///
/// - The current object was **identified as `T`**. The
///   [`ExtractionContext`] then covers the whole object
///   (`offset = 0`, `length = Some(content.size())`, `params = None`).
/// - An analyzer **requested** extraction of `T` from the current
///   object via [`Context::request_extract`]. The context then carries
///   the requested offset, length, and params. The parent does not
///   need to have been identified as `T`.
///
/// One extractor instance is shared by every object of its type, and
/// `create_session` can be re-entered while a previous session is
/// still live (nested containers). Keep configuration on the extractor
/// and put cursors, open archives, and the current [`Entry`] on the
/// session.
///
/// Register extractors via
/// [`ScannerBuilder::add_extractor`](crate::ScannerBuilder::add_extractor).
/// Multiple extractors for the same type run in registration order.
pub trait ContentExtractor<T: ContentType> {
    /// Opens an extraction session on `content`.
    ///
    /// `content` is a non-owning handle to the parent. Store it on the
    /// session if [`advance`](ExtractionSession::advance) /
    /// [`extract`](ExtractionSession::extract) need to read the parent;
    /// it stays valid until the session is dropped.
    ///
    /// `extract_context` names the region of the parent to look at and
    /// is only valid for this call — copy [`ExtractionContext::offset`],
    /// [`ExtractionContext::length`], and any
    /// [`ExtractionContext::params`] you need into the session.
    ///
    /// Return `Some(session)` to start enumerating children, or `None`
    /// to skip this extractor (the scanner moves on to the next one
    /// registered for the same type). The session is dropped when
    /// enumeration ends, when a nested child's analyzer returns
    /// [`NextAction::Exit`], or when the scanner moves on — implement
    /// [`Drop`] on the session if you need to close files or free
    /// buffers.
    fn create_session(&mut self, content: OwnedContentPtr<T>, extract_context: &ExtractionContext) -> Option<Box<dyn ExtractionSession<T>>>;
}

/// A live extraction of children from one parent [`Content`].
///
/// Produced by [`ContentExtractor::create_session`] and driven by the
/// scanner as a short loop:
///
/// 1. [`advance`](Self::advance) — move to the next child and return
///    a lightweight [`Entry`] (path / size / filter skip). `None`
///    ends the stream and the session is dropped.
/// 2. [`extract`](Self::extract) — materialize that child as a boxed
///    [`Content`]. The scanner then recursively scans it (subject to
///    `max_depth`). `None` skips just this entry; enumeration
///    continues with the next `advance`.
///
/// A child's [`NextAction::Skip`] does not end this session — the
/// scanner continues with the next `advance`. A child's
/// [`NextAction::Exit`] drops the session immediately as the scan
/// unwinds.
///
/// Keep one [`Entry`] as a field and overwrite `entry.path` in place
/// with [`ContentPath::set_from_str`](crate::ContentPath::set_from_str)
/// (synthetic names) or
/// [`ContentPath::set_from_os`](crate::ContentPath::set_from_os)
/// (real OS paths) so `advance` does not allocate a new path for
/// every child.
pub trait ExtractionSession<T: ContentType> {
    /// Advances to the next child and returns its [`Entry`].
    ///
    /// Returning `None` ends the stream. The returned reference must
    /// remain valid until the next call to `advance` or `extract`, or
    /// until the session is dropped — typically it borrows a field on
    /// `self`.
    fn advance(&mut self) -> Option<&Entry>;

    /// Materializes the child announced by the last [`advance`](Self::advance).
    ///
    /// Returning `None` skips this entry; the scanner will call
    /// `advance` again. The boxed content is scanned recursively
    /// before the next `advance`.
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
/// The scanner only feeds the matcher the first 16 bytes of content,
/// and only when at least one identifier registered such a pattern;
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
    /// That 16-byte read is performed only when at least one identifier
    /// registered a [`Magic`](Self::Magic) or
    /// [`MultipleMagic`](Self::MultipleMagic) pattern.
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

/// Debug-only name and required-analyzer list for a plugin.
///
/// [`ContentAnalyzer`] requires this trait. The usual implementation
/// is `#[derive(Dependencies)]` with a helper attribute:
///
/// ```ignore
/// use content_scan::Dependencies;
///
/// #[derive(Dependencies)]
/// #[Dependencies(name = "NeedsHash", requires = "ComputeHash")]
/// struct NeedsHash;
/// ```
///
/// - `name` is required and must be a non-empty string. It is the
///   identifier other analyzers use in `requires`.
/// - `requires` is optional. It may be a single string or an array of
///   strings naming other analyzers that must run first.
///
/// `name()` and `dependencies()` exist only when `debug_assertions`
/// are enabled. In debug builds,
/// [`ScannerBuilder::build`](crate::ScannerBuilder::build) verifies
/// that every required name is a registered analyzer and that each
/// dependency has a **strictly smaller** `priority`. The check is
/// global: typed and generic analyzers share one name space,
/// regardless of [`ContentType`].
pub trait Dependencies {
    /// Unique name of this analyzer.
    ///
    /// Only available in debug builds. Used by
    /// [`ScannerBuilder::build`](crate::ScannerBuilder::build) to
    /// resolve `requires` entries.
    #[cfg(debug_assertions)]
    fn name(&self) -> &'static str;

    /// Names of analyzers this one requires, matching their [`name`](Self::name).
    ///
    /// Only available in debug builds. Empty when `requires` was
    /// omitted. Each listed analyzer must be registered with a
    /// strictly smaller `priority` than this one.
    #[cfg(debug_assertions)]
    fn dependencies(&self) -> &'static [&'static str];
}