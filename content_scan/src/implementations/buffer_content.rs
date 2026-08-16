use crate::{Content, ContentPath, ContentType};

/// An in-memory [`Content`] backed by an owned byte buffer.
///
/// `BufferContent` is the simplest way to feed data to a
/// [`Scanner`](crate::Scanner): construct it from a byte slice (or an
/// owned `Vec<u8>`), give it a path, and hand it to
/// [`Scanner::scan`](crate::Scanner::scan). Extractors typically return
/// `BufferContent` instances to represent nested items.
pub struct BufferContent<T: ContentType> {
    buffer: Vec<u8>,
    path: ContentPath,
    content_type: Option<T>,
}
impl<T: ContentType> BufferContent<T> {
    /// Creates a new `BufferContent` by copying `buffer`.
    ///
    /// `path` is a synthetic UTF-8 address stored via
    /// [`ContentPath::from_str`]. The content type is left unset, so
    /// the scanner will identify it automatically using magic bytes,
    /// file name, or extension.
    pub fn new(buffer: &[u8], path: &str) -> Self {
        Self {
            buffer: buffer.to_vec(),
            path: ContentPath::from_str(path),
            content_type: None,
        }
    }

    /// Creates a new `BufferContent` by copying `buffer` and pinning it
    /// to a specific content type.
    ///
    /// Because the type is known up front, the scanner will not attempt
    /// to identify it and will dispatch directly to the analyzers and
    /// extractors registered for `content_type`.
    pub fn with_content_type(buffer: &[u8], path: &str, content_type: T) -> Self {
        Self {
            buffer: buffer.to_vec(),
            path: ContentPath::from_str(path),
            content_type: Some(content_type),
        }
    }

    /// Creates a `BufferContent` from already-owned parts, avoiding a
    /// copy of the buffer.
    ///
    /// Use this constructor when you already own a `Vec<u8>` and a
    /// UTF-8 path `String` and want to move them into the
    /// `BufferContent` without paying an extra allocation. The path is
    /// stored as a lossless [`ContentPath`]. Passing
    /// `content_type = None` lets the scanner identify the type
    /// automatically.
    pub fn from_parts(buffer: Vec<u8>, path: String, content_type: Option<T>) -> Self {
        Self {
            buffer,
            path: ContentPath::with_string(path),
            content_type,
        }
    }
}
impl<T: ContentType> Content<T> for BufferContent<T> {
    #[inline(always)]
    fn content_type(&self) -> Option<T> {
        self.content_type
    }
    #[inline(always)]
    fn path(&self) -> &ContentPath {
        &self.path
    }
    #[inline(always)]
    fn size(&self) -> u64 {
        self.buffer.len() as u64
    }
    fn read(&mut self, offset: u64, count: u32) -> Option<&[u8]> {
        if offset > self.buffer.len() as u64 {
            return None;
        }
        if offset == self.buffer.len() as u64 {
            return Some(&[]);
        }
        let len = (self.buffer.len() as u64 - offset).min(count as u64) as usize;
        Some(&self.buffer.as_slice()[offset as usize..offset as usize + len])
    }
}
