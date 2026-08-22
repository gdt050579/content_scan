use crate::ContentPath;
use std::{
    fmt::Debug,
    io::{Read, Seek, SeekFrom},
    ops::{Deref, DerefMut},
};

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

#[derive(Debug, Copy, Clone)]
pub(crate) struct ContentPtr<T: ContentType> {
    content: *mut dyn Content<T>,
}
impl<T: ContentType> ContentPtr<T> {
    #[inline(always)]
    pub(crate) fn new(content: &mut dyn Content<T>) -> Self {
        Self {
            content: unsafe { std::mem::transmute::<*mut dyn Content<T>, *mut (dyn Content<T> + 'static)>(content as *mut _) },
        }
    }
    #[inline(always)]
    pub(crate) fn as_mut(&mut self) -> &mut dyn Content<T> {
        unsafe { &mut *self.content }
    }
    #[inline(always)]
    pub(crate) fn as_ref(&self) -> &dyn Content<T> {
        unsafe { &*self.content }
    }
}
/// Non-owning handle to the parent [`Content`] of an extraction session.
///
/// [`ContentExtractor::create_session`](crate::ContentExtractor::create_session)
/// receives this instead of a `&mut dyn Content<T>` so the session can
/// keep the parent across [`advance`](crate::ExtractionSession::advance)
/// / [`extract`](crate::ExtractionSession::extract) without fighting
/// the borrow checker. It [derefs](std::ops::Deref) to `dyn Content<T>`,
/// so a session can call [`path`](Content::path), [`size`](Content::size),
/// [`read`](Content::read), and [`content_type`](Content::content_type)
/// through it.
///
/// The handle is valid for the lifetime of the session that received
/// it. The scanner guarantees the parent outlives that session; do not
/// store the pointer after the session is dropped.
pub struct OwnedContentPtr<T: ContentType> {
    content: *mut dyn Content<T>,
}
impl<T: ContentType> OwnedContentPtr<T> {
    #[inline(always)]
    pub(crate) fn new(content: ContentPtr<T>) -> Self {
        Self { content: content.content }
    }
}
impl<T: ContentType> Deref for OwnedContentPtr<T> {
    type Target = dyn Content<T>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.content }
    }
}
impl<T: ContentType> DerefMut for OwnedContentPtr<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.content }
    }
}

/// Sequential [`Read`] + [`Seek`] adapter over an [`OwnedContentPtr`].
///
/// [`Content::read`] is random-access and returns a borrowed slice.
/// Libraries that expect a `std::io::Read` / `Seek` stream (for
/// example the `zip` crate used by [`ZipExtractor`](crate::ZipExtractor))
/// need a cursor instead. `ContentReader` wraps the parent handle,
/// starts at offset `0`, and advances on each `read`.
///
/// A short slice from the underlying [`Content`] is not treated as
/// EOF: the adapter copies what it got, advances, and the next
/// `Read::read` continues from there. A `Content::read` that returns
/// `None` before the advertised [`size`](Content::size) becomes an
/// `UnexpectedEof` error. Seeking past the end is allowed (same as
/// [`std::io::Cursor`]); a later read then returns `Ok(0)`.
///
/// ```ignore
/// let mut reader = ContentReader::new(parent);
/// let mut archive = zip::ZipArchive::new(reader)?;
/// ```
pub struct ContentReader<T: ContentType> {
    content: OwnedContentPtr<T>,
    offset: u64,
}
impl<T: ContentType> ContentReader<T> {
    /// Wraps `content` with the read cursor at offset `0`.
    pub fn new(content: OwnedContentPtr<T>) -> Self {
        Self { content, offset: 0 }
    }
}
impl<T: ContentType> Read for ContentReader<T> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let len = self.content.size();
        if self.offset >= len {
            return Ok(0); // at or past EOF: clean end of stream
        }

        let remaining = len - self.offset;
        let want = (buf.len() as u64).min(remaining).min(u32::MAX as u64) as u32;

        match self.content.read(self.offset, want) {
            Some(src) => {
                let n = src.len().min(buf.len());
                buf[..n].copy_from_slice(&src[..n]);
                self.offset += n as u64;
                Ok(n)
            }
            None => Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Content::read failed before end of content",
            )),
        }
    }
}

impl<T: ContentType> Seek for ContentReader<T> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let (base, delta) = match pos {
            SeekFrom::Start(o) => {
                self.offset = o;
                return Ok(o);
            }
            SeekFrom::End(o) => (self.content.size(), o),
            SeekFrom::Current(o) => (self.offset, o),
        };

        let new = if delta >= 0 {
            base.checked_add(delta as u64)
        } else {
            base.checked_sub(delta.unsigned_abs())
        };

        match new {
            Some(p) => {
                self.offset = p;
                Ok(p)
            }
            None => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "seek to a negative or overflowing position",
            )),
        }
    }
}
