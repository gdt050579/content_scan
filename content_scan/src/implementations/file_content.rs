use crate::{Content, ContentType, ContentPath};
use filecache::{FileCache, CacheType, RandomAccessFile, RandomAccessFlags};
use std::{fs, path::Path};

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
    path: ContentPath,
    content_type: Option<T>,    
    status: FileContentStatus,
    size: u64,
}
impl<T: ContentType> FileContent<T> {
    /// Creates a `FileContent` for `path`, querying its size upfront.
    ///
    /// The path is stored via [`ContentPath::from_os`], so a non-UTF-8
    /// filesystem name stays openable. The content type is left unset,
    /// so the scanner identifies it automatically. A path that cannot
    /// be stat'ed yields a size of `0`.
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: ContentPath::from_os(path.as_ref()),
            content_type: None,
            status: FileContentStatus::NotOpened,
            size: fs::metadata(path).map(|m| m.len()).unwrap_or(0),
        }
    }

    /// Creates a `FileContent` pinned to a known content type.
    ///
    /// The scanner skips identification and dispatches directly to the
    /// plugins registered for `content_type`.
    pub fn with_content_type(path: impl AsRef<Path>, content_type: T) -> Self {
        Self {
            path: ContentPath::from_os(path.as_ref()),
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
    pub fn with_size(path: impl AsRef<Path>, size: u64) -> Self {
        Self {
            path: ContentPath::from_os(path.as_ref()),
            content_type: None,
            status: FileContentStatus::NotOpened,
            size,
        }
    }
    fn open(&mut self) {
        match RandomAccessFile::open(self.path.as_path(), RandomAccessFlags::Exclusive) {
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
    fn path(&self) -> &ContentPath {
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
