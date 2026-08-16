use super::{FileContent, FolderContent};
use crate::{Content, ContentExtractor, ContentType, ExtractionContext, ExtractionPool};
use std::marker::PhantomData;

/// Returns `true` if the directory entry is itself a link (not its target).
///
/// On Unix this is a plain symlink. On Windows it also covers junctions
/// and other reparse points, which [`std::fs::FileType::is_symlink`] does
/// **not** report (it only matches `IO_REPARSE_TAG_SYMLINK`). Neither
/// [`FileType::is_symlink`] nor [`DirEntry::metadata`] follows the link,
/// so this inspects the link entry, not what it points to.
#[cfg(windows)]
fn is_link(ft: &std::fs::FileType, ent: &std::fs::DirEntry) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    match ent.metadata() {
        Ok(md) => md.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0,
        Err(_) => ft.is_symlink(), // fallback
    }
}

#[cfg(not(windows))]
fn is_link(ft: &std::fs::FileType, _ent: &std::fs::DirEntry) -> bool {
    ft.is_symlink()
}

/// A [`ContentExtractor`] that enumerates the entries of a directory.
///
/// Register it for the same content type the parent
/// [`FolderContent`] carries, and the scanner will walk the directory
/// tree for you:
///
/// ```ignore
/// let mut scanner = ScannerBuilder::<MyTypes>::new()
///     .add_extractor(MyTypes::Folder, FolderExtractor::<MyTypes>::new(true, false))
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
/// - Entries that cannot be read (permission errors, broken
///   `file_type()`) are skipped; the rest of the directory is still
///   enumerated.
/// - Symbolic links whose target is a directory are skipped, so link
///   cycles cannot make the walk loop forever. On Windows this also
///   covers directory junctions and other reparse points. Symbolic
///   links to regular files are followed and emitted like any other
///   file; broken (dangling) links are skipped.
/// - Subdirectory entries set
///   [`Entry::skip_from_filtering`](crate::Entry::skip_from_filtering),
///   so a [`Filter`](crate::Filter) restricted to certain file
///   extensions narrows the files without blocking the descent.
/// - Depth is bounded by the scanner's
///   [`max_depth`](crate::ScannerBuilder::max_depth), which here
///   counts directory nesting levels.
pub struct FolderExtractor<T: ContentType> {
    _marker: PhantomData<T>,
    pool: ExtractionPool<std::fs::ReadDir>,
    entry: crate::Entry,
    current_is_folder: bool,
    recursive: bool,
    open_files_exclusively: bool,
}
impl<T: ContentType> FolderExtractor<T> {
    /// Creates a folder extractor.
    ///
    /// When `recursive` is `false`, subdirectories are not emitted at
    /// all and only the files directly inside the parent folder are
    /// scanned. `open_files_exclusively` is forwarded to
    /// [`FileContent::with_size`]: `true` memory-maps with an exclusive
    /// lock, `false` uses shared LRU reads.
    pub fn new(recursive: bool, open_files_exclusively: bool) -> Self {
        Self {
            _marker: PhantomData,
            pool: ExtractionPool::new(4),
            entry: crate::Entry::default(),
            current_is_folder: false,
            recursive,
            open_files_exclusively,
        }
    }
}
impl<T: ContentType + 'static> ContentExtractor<T> for FolderExtractor<T> {
    fn acquire(&mut self, content: &mut dyn Content<T>, _: &ExtractionContext) -> Option<crate::ExtractionHandle> {
        let obj = std::fs::read_dir(content.path().as_path()).ok()?;
        Some(self.pool.acquire_slot(obj))
    }

    fn advance(&mut self, handle: crate::ExtractionHandle, _: &mut dyn Content<T>) -> Option<&crate::Entry> {
        let rd = self.pool.get_mut(handle)?;
        loop {
            let folder_ent = match rd.next() {
                Some(Ok(ent)) => ent,
                Some(Err(_)) => continue, // skip unreadable entries
                None => return None,
            };
            let ft = match folder_ent.file_type() {
                Ok(ft) => ft,
                Err(_) => continue, // skip entries whose type cannot be determined
            };

            // Whether this entry is a link (symlink / reparse point / junction),
            // and — for links — what the *target* is. We must follow the link with
            // `std::fs::metadata` to learn the target type, since `file_type()` and
            // `DirEntry::metadata()` describe the link itself, not its target.
            let is_link = is_link(&ft, &folder_ent);
            let mut link_target_len: Option<u64> = None;

            if is_link {
                match std::fs::metadata(&folder_ent.path()) {
                    // Symlink -> folder: skip, to avoid circular references.
                    Ok(target) if target.is_dir() => continue,
                    // Symlink -> file: keep, and remember the target size.
                    Ok(target) => link_target_len = Some(target.len()),
                    // Broken / dangling link: skip.
                    Err(_) => continue,
                }
                // A kept link always refers to a regular file here.
                self.current_is_folder = false;
            } else {
                self.current_is_folder = ft.is_dir();
            }

            if !self.recursive && self.current_is_folder {
                continue;
            } // skip folders if not recursive

            self.entry.path.clear();
            self.entry.path.set_from_os(&folder_ent.path());
            self.entry.size = if self.current_is_folder {
                0
            } else if let Some(len) = link_target_len {
                // File symlink: size comes from the followed target, since
                // `DirEntry::metadata()` would report the link's own size.
                len
            } else {
                folder_ent.metadata().map(|m| m.len()).unwrap_or(0)
            };
            self.entry.skip_from_filtering = self.current_is_folder;
            return Some(&self.entry);
        }
    }

    fn extract(&mut self, _: crate::ExtractionHandle, content: &mut dyn Content<T>) -> Option<Box<dyn Content<T>>> {
        if self.current_is_folder {
            Some(Box::new(FolderContent::with_content_type(
                self.entry.path.as_path(),
                content.content_type()?,
            )))
        } else {
            Some(Box::new(FileContent::with_size(
                self.entry.path.as_path(),
                self.entry.size,
                self.open_files_exclusively,
            )))
        }
    }

    fn release(&mut self, handle: crate::ExtractionHandle) {
        self.pool.release_slot(handle);
    }
}
