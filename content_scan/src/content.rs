use filecache::*;
use std::{fmt::Debug, fs, marker::PhantomData, path::Path};

use crate::{ContentExtractor, ExtractionPool};

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
/// #[derive(Debug, Copy, Clone, Eq, PartialEq, ContentType)]
/// #[repr(u16)]
/// enum MyTypes { Text, Binary }
/// ```
pub trait ContentType: Copy + Eq + PartialEq + Debug {
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
/// Implementors expose a virtual path, a total size, and a way to read
/// arbitrary byte ranges. Analyzers and extractors receive a `&mut dyn
/// Content<T>` so they can request the exact windows they need without
/// requiring the whole payload to be buffered in memory up front.
///
/// [`BufferContent`] is provided as a ready-to-use in-memory
/// implementation; you can implement `Content` for your own types (for
/// example a memory-mapped file, a network stream wrapper, or an entry
/// inside an archive).
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

    /// Returns the virtual path associated with this content.
    ///
    /// The path is used by [`Filter`](crate::Filter) rules, stored in
    /// the scan result tree, and is available to analyzers and
    /// extractors. It does not need to correspond to a real filesystem
    /// path.
    fn path(&self) -> &str;

    /// Returns the total size of the content in bytes.
    fn size(&self) -> u64;

    /// Reads up to `count` bytes starting at `offset`.
    ///
    /// - Returns `Some(&[])` if `offset` is exactly at the end of the
    ///   content.
    /// - Returns `None` if `offset` is past the end or the underlying
    ///   source cannot fulfill the request.
    /// - May return fewer bytes than requested when the request runs
    ///   past the end of the content.
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

/// An in-memory [`Content`] backed by an owned byte buffer.
///
/// `BufferContent` is the simplest way to feed data to a
/// [`Scanner`](crate::Scanner): construct it from a byte slice (or an
/// owned `Vec<u8>`), give it a path, and hand it to
/// [`Scanner::scan`](crate::Scanner::scan). Extractors typically return
/// `BufferContent` instances to represent nested items.
pub struct BufferContent<T: ContentType> {
    buffer: Vec<u8>,
    path: String,
    content_type: Option<T>,
}
impl<T: ContentType> BufferContent<T> {
    /// Creates a new `BufferContent` by copying `buffer`.
    ///
    /// The content type is left unset, so the scanner will identify it
    /// automatically using magic bytes, file name, or extension.
    pub fn new(buffer: &[u8], path: &str) -> Self {
        Self {
            buffer: buffer.to_vec(),
            path: path.to_string(),
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
            path: path.to_string(),
            content_type: Some(content_type),
        }
    }

    /// Creates a `BufferContent` from already-owned parts, avoiding a
    /// copy of the buffer.
    ///
    /// Use this constructor when you already own a `Vec<u8>` and a
    /// `String` and want to move them into the `BufferContent` without
    /// paying an extra allocation. Passing `content_type = None` lets
    /// the scanner identify the type automatically.
    pub fn from_parts(buffer: Vec<u8>, path: String, content_type: Option<T>) -> Self {
        Self { buffer, path, content_type }
    }
}
impl<T: ContentType> Content<T> for BufferContent<T> {
    #[inline(always)]
    fn content_type(&self) -> Option<T> {
        self.content_type
    }
    #[inline(always)]
    fn path(&self) -> &str {
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

enum FileContentStatus {
    NotOpened,
    Opened(FileCache<filecache::RandomAccessFile>),
    Error,
}
pub struct FileContent<T: ContentType> {
    path: String,
    content_type: Option<T>,
    status: FileContentStatus,
    size: u64,
}
impl<T: ContentType> FileContent<T> {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
            content_type: None,
            status: FileContentStatus::NotOpened,
            size: fs::metadata(path).map(|m| m.len()).unwrap_or(0),
        }
    }
    pub fn with_content_type(path: &str, content_type: T) -> Self {
        Self {
            path: path.to_string(),
            content_type: Some(content_type),
            status: FileContentStatus::NotOpened,
            size: fs::metadata(path).map(|m| m.len()).unwrap_or(0),
        }
    }
    fn open(&mut self) {
        match RandomAccessFile::open(Path::new(&self.path), RandomAccessFlags::Exclusive) {
            Ok(reader) => match FileCache::new(CacheType::MemoryMap, reader) {
                Ok(file) => {
                    self.status = FileContentStatus::Opened(file);
                }
                Err(_) => {
                    self.status = FileContentStatus::Error;
                }
            },
            Err(_) => {
                self.status = FileContentStatus::Error;
            }
        }
    }
}
impl<T: ContentType> Content<T> for FileContent<T> {
    #[inline(always)]
    fn content_type(&self) -> Option<T> {
        self.content_type
    }

    #[inline(always)]
    fn path(&self) -> &str {
        &self.path
    }

    #[inline(always)]
    fn size(&self) -> u64 {
        match &self.status {
            FileContentStatus::Opened(file) => file.len(),
            FileContentStatus::NotOpened => self.size,
            FileContentStatus::Error => 0,
        }
    }

    #[inline(always)]
    fn read(&mut self, offset: u64, count: u32) -> Option<&[u8]> {
        if matches!(self.status, FileContentStatus::NotOpened) {
            self.open();
        }
        match &mut self.status {
            FileContentStatus::Opened(file) => file.read(offset, count as usize).ok(),
            FileContentStatus::NotOpened | FileContentStatus::Error => None,
        }
    }
}
pub struct FolderContent<T: ContentType> {
    path: String,
    content_type: T,
}
impl<T: ContentType> FolderContent<T> {
    pub fn with_content_type(path: &str, content_type: T) -> Self {
        Self {
            path: path.to_string(),
            content_type,
        }
    }
}
impl<T: ContentType> Content<T> for FolderContent<T> {
    fn path(&self) -> &str {
        &self.path
    }

    fn size(&self) -> u64 {
        0
    }

    fn read(&mut self, _: u64, _: u32) -> Option<&[u8]> {
        None
    }

    fn content_type(&self) -> Option<T> {
        Some(self.content_type)
    }
}
pub struct FolderExtractor<T: ContentType> {
    _marker: PhantomData<T>,
    pool: ExtractionPool<fs::ReadDir>,
    entry: crate::Entry,
    current_is_folder: bool,
}
impl<T: ContentType> FolderExtractor<T> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
            pool: ExtractionPool::new(4),
            entry: crate::Entry::default(),
            current_is_folder: false
        }
    }
}
impl<T: ContentType + 'static> ContentExtractor<T> for FolderExtractor<T> {
    fn acquire(&mut self, content: &mut dyn Content<T>, _: &mut varmap::VarMap) -> Option<crate::ExtractionHandle> {
        let obj = fs::read_dir(content.path()).ok()?;
        Some(self.pool.acquire_slot(obj))
    }

    fn advance(&mut self, handle: crate::ExtractionHandle, _: &mut dyn Content<T>) -> Option<&crate::Entry> {
        let Some(rd) = self.pool.get_mut(handle) else {
            return None;
        };
        loop {
            let folder_ent = rd.next()?.ok()?;
            let ft = folder_ent.file_type().ok()?;
            self.current_is_folder = ft.is_dir();
            let symlink = ft.is_symlink();
            if self.current_is_folder && symlink { continue; } // skip directory symlinks
            self.entry.path.clear();
            // to review (no allocation)
            self.entry.path.push_str(folder_ent.path().to_str().unwrap_or_default());
            self.entry.size = 0; // folder
            return Some(&self.entry);
        }
    }

    fn extract(&mut self, _: crate::ExtractionHandle, content: &mut dyn Content<T>) -> Option<Box<dyn Content<T>>> {
        if self.current_is_folder {
            Some(Box::new(FolderContent::with_content_type(content.path(), content.content_type()?)))
        } else {
            Some(Box::new(FileContent::new(content.path())))
        }
    }

    fn release(&mut self, handle: crate::ExtractionHandle) {
        self.pool.release_slot(handle);
    }
}
