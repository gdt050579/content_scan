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

/// A [`Content`] backed by a file on disk.
///
/// The file is opened and memory-mapped lazily, on the first
/// [`read`](Content::read); constructing a `FileContent` for a file
/// that is never read costs nothing but the path. A file that cannot
/// be opened behaves like an empty content: `size()` is `0` and every
/// read returns `None`.
pub struct FileContent<T: ContentType> {
    path: String,
    content_type: Option<T>,
    status: FileContentStatus,
    size: u64,
}
impl<T: ContentType> FileContent<T> {
    /// Creates a `FileContent` for `path`, querying its size upfront.
    ///
    /// The content type is left unset, so the scanner identifies it
    /// automatically. A path that cannot be stat'ed yields a size of
    /// `0`.
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
            content_type: None,
            status: FileContentStatus::NotOpened,
            size: fs::metadata(path).map(|m| m.len()).unwrap_or(0),
        }
    }

    /// Creates a `FileContent` pinned to a known content type.
    ///
    /// The scanner skips identification and dispatches directly to the
    /// plugins registered for `content_type`.
    pub fn with_content_type(path: &str, content_type: T) -> Self {
        Self {
            path: path.to_string(),
            content_type: Some(content_type),
            status: FileContentStatus::NotOpened,
            size: fs::metadata(path).map(|m| m.len()).unwrap_or(0),
        }
    }

    /// Creates a `FileContent` with a size the caller already knows.
    ///
    /// Unlike [`new`](Self::new), this does not touch the filesystem at
    /// construction time. Use it when the size comes from something
    /// that has already been read — a directory entry's metadata, an
    /// index, a manifest — to avoid a redundant `stat`.
    pub fn with_size(path: &str, size: u64) -> Self {
        Self {
            path: path.to_string(),
            content_type: None,
            status: FileContentStatus::NotOpened,
            size,
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
/// A [`Content`] standing for a directory rather than a byte stream.
///
/// It carries only a path: [`size`](Content::size) is always `0` and
/// [`read`](Content::read) always returns `None`. Its purpose is to
/// give the scanner an object it can dispatch on, so that a
/// [`FolderExtractor`] registered for the same content type can
/// enumerate the directory's entries.
///
/// The content type is mandatory (there is nothing to identify a
/// directory by), and callers supply a variant of their own enum:
///
/// ```ignore
/// let mut root = FolderContent::<MyTypes>::with_content_type("./src", MyTypes::Folder);
/// let result = scanner.scan(&mut root, false);
/// ```
pub struct FolderContent<T: ContentType> {
    path: String,
    content_type: T,
}
impl<T: ContentType> FolderContent<T> {
    /// Creates a `FolderContent` for `path`, tagged as `content_type`.
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
/// A [`ContentExtractor`] that enumerates the entries of a directory.
///
/// Register it for the same content type the parent
/// [`FolderContent`] carries, and the scanner will walk the directory
/// tree for you:
///
/// ```ignore
/// let mut scanner = ScannerBuilder::<MyTypes>::new()
///     .add_extractor(MyTypes::Folder, 0, FolderExtractor::<MyTypes>::new(true))
///     .build();
/// ```
///
/// Each entry becomes a child content object: files are emitted as
/// [`FileContent`] built with [`FileContent::with_size`] (reusing the
/// size from the directory entry, so no extra `stat` is needed), and
/// subdirectories as [`FolderContent`] carrying the parent's content
/// type — which is what makes the walk recurse through this same
/// extractor.
///
/// Notable behaviour:
///
/// - Symbolic links to directories are skipped, so link cycles cannot
///   make the walk loop forever.
/// - Subdirectory entries set
///   [`Entry::skip_from_filtering`](crate::Entry::skip_from_filtering),
///   so a [`Filter`](crate::Filter) restricted to certain file
///   extensions narrows the files without blocking the descent.
/// - Depth is bounded by the scanner's
///   [`max_depth`](crate::ScannerBuilder::max_depth), which here
///   counts directory nesting levels.
pub struct FolderExtractor<T: ContentType> {
    _marker: PhantomData<T>,
    pool: ExtractionPool<fs::ReadDir>,
    entry: crate::Entry,
    current_is_folder: bool,
    recursive: bool,
}
impl<T: ContentType> FolderExtractor<T> {
    /// Creates a folder extractor.
    ///
    /// When `recursive` is `false`, subdirectories are not emitted at
    /// all and only the files directly inside the parent folder are
    /// scanned.
    pub fn new(recursive: bool) -> Self {
        Self {
            _marker: PhantomData,
            pool: ExtractionPool::new(4),
            entry: crate::Entry::default(),
            current_is_folder: false,
            recursive,
        }
    }
}
impl<T: ContentType + 'static> ContentExtractor<T> for FolderExtractor<T> {
    fn acquire(&mut self, content: &mut dyn Content<T>, _: &mut varmap::VarMap) -> Option<crate::ExtractionHandle> {
        let obj = fs::read_dir(content.path()).ok()?;
        Some(self.pool.acquire_slot(obj))
    }

    fn advance(&mut self, handle: crate::ExtractionHandle, _: &mut dyn Content<T>) -> Option<&crate::Entry> {
        let rd = self.pool.get_mut(handle)?;
        loop {
            let folder_ent = rd.next()?.ok()?;
            let ft = folder_ent.file_type().ok()?;
            self.current_is_folder = ft.is_dir();
            let symlink = ft.is_symlink();
            if self.current_is_folder && symlink {
                continue;
            } // skip directory symlinks
            if !self.recursive && self.current_is_folder {
                continue;
            } // skip folders if not recursive
            self.entry.path.clear();
            // to review (no allocation)
            self.entry.path.push_str(folder_ent.path().to_str().unwrap_or_default());
            self.entry.size = if self.current_is_folder { 0 } else { folder_ent.metadata().map(|m| m.len()).unwrap_or(0) }; // folder
            self.entry.skip_from_filtering = self.current_is_folder;
            return Some(&self.entry);
        }
    }

    fn extract(&mut self, _: crate::ExtractionHandle, content: &mut dyn Content<T>) -> Option<Box<dyn Content<T>>> {
        if self.current_is_folder {
            Some(Box::new(FolderContent::with_content_type(&self.entry.path, content.content_type()?)))
        } else {
            Some(Box::new(FileContent::with_size(&self.entry.path, self.entry.size)))
        }
    }

    fn release(&mut self, handle: crate::ExtractionHandle) {
        self.pool.release_slot(handle);
    }
}
