use crate::ContentPath;
use std::fmt::Debug;

/// Enumeration of content kinds understood by a scanner.
///
/// A `ContentType` is typically a user-defined `#[repr(u16)]` enum
/// derived with `#[derive(ContentType)]` (see
/// [`content_scan_proc_macro::ContentType`]). It maps each variant to a
/// stable `u16` identifier that the framework uses internally for fast
/// dispatch to identifiers, analyzers, and extractors.
///
/// # Contract
///
/// - [`as_u16`](Self::as_u16) must return a stable value that is unique
///   per variant and strictly less than [`COUNT`](Self::COUNT).
/// - [`from_u16`](Self::from_u16) must be the inverse of `as_u16` and
///   return `None` for values that do not correspond to a variant.
/// - [`COUNT`](Self::COUNT) is the number of possible variants and is
///   used to size internal fast-lookup tables.
///
/// # Example
///
/// ```ignore
/// #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ContentType)]
/// #[repr(u16)]
/// enum MyTypes { Text, Binary }
/// ```
pub trait ContentType: Copy + Eq + PartialEq + Debug + Ord + PartialOrd {
    /// Total number of variants of this type.
    ///
    /// This must be strictly greater than the largest value ever
    /// returned by [`as_u16`](Self::as_u16). It is used to preallocate
    /// dispatch tables inside the scanner.
    const COUNT: u16;

    /// Returns the stable `u16` identifier for this variant.
    ///
    /// The value must be less than [`COUNT`](Self::COUNT) and unique
    /// among variants.
    fn as_u16(&self) -> u16;

    /// Reconstructs a variant from its `u16` identifier.
    ///
    /// Returns `None` if `value` does not correspond to any variant of
    /// this type.
    fn from_u16(value: u16) -> Option<Self>;
}

/// A byte-addressable piece of content the scanner can operate on.
///
/// Implementors expose a [`ContentPath`], a total size, and a way to
/// read arbitrary byte ranges. Analyzers and extractors receive a
/// `&mut dyn Content<T>` so they can request the exact windows they
/// need without requiring the whole payload to be buffered in memory
/// up front.
///
/// Ready-made implementations: [`crate::BufferContent`] (in-memory),
/// [`crate::FileContent`] (a file on disk), and [`crate::FolderContent`]
/// (a directory). You can also implement `Content` for your own types
/// (for example a memory-mapped region, a network stream wrapper, or
/// an entry inside an archive).
pub trait Content<T: ContentType> {
    /// Returns the content type if it is already known.
    ///
    /// When this returns `Some(ty)` the scanner will skip its own type
    /// detection (magic / extension / file name) and use `ty` directly.
    /// The default implementation returns `None`, meaning the scanner
    /// must identify the type itself.
    fn content_type(&self) -> Option<T> {
        None
    }

    /// Returns the path associated with this content.
    ///
    /// The path is a [`ContentPath`]: a UTF-8 printable view always
    /// exists, and a real OS path (including non-UTF-8 names) stays
    /// openable via [`ContentPath::as_path`]. It is used by
    /// [`Filter`](crate::Filter) rules, stored in the scan result tree,
    /// and available to analyzers and extractors. It does not need to
    /// correspond to a real filesystem path — archive members and
    /// in-memory buffers typically use a synthetic address built with
    /// [`ContentPath::from_str`].
    fn path(&self) -> &ContentPath;

    /// Returns the total size of the content in bytes.
    fn size(&self) -> u64;

    /// Reads up to `count` bytes starting at `offset`.
    ///
    /// - Returns `Some(&[])` if `offset` is exactly at the end of the
    ///   content.
    /// - Returns `None` if `offset` is past the end or the underlying
    ///   source cannot fulfill the request.
    /// - May return fewer bytes than requested at EOF **or** at an
    ///   implementation boundary (for example one cache page). A short
    ///   slice is not end of file; advance by its length and read again.
    ///
    /// The returned slice borrows from `self`, so the read cursor is
    /// implicit: successive calls with different `offset` values do not
    /// interact.
    fn read(&mut self, offset: u64, count: u32) -> Option<&[u8]>;
}

/// Trivial [`ContentType`] implementation for `bool`.
///
/// This is convenient when a scanner only needs to distinguish between
/// two abstract kinds of content (`false` and `true`) or when writing
/// tests. `false` maps to `0`, `true` to `1`, and `COUNT` is `2`.
impl ContentType for bool {
    const COUNT: u16 = 2;
    fn as_u16(&self) -> u16 {
        *self as u16
    }
    fn from_u16(value: u16) -> Option<Self> {
        if value == 0 {
            Some(false)
        } else if value == 1 {
            Some(true)
        } else {
            None
        }
    }
}
