use crate::{Content, ContentExtractor, ContentType, ExtractionPool};
use std::marker::PhantomData;
use super::{FolderContent, FileContent};

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
    pool: ExtractionPool<std::fs::ReadDir>,
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
        let obj = std::fs::read_dir(content.path().as_path()).ok()?;
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
            self.entry.path.set_from_os(&folder_ent.path());
            self.entry.size = if self.current_is_folder { 0 } else { folder_ent.metadata().map(|m| m.len()).unwrap_or(0) }; // folder
            self.entry.skip_from_filtering = self.current_is_folder;
            return Some(&self.entry);
        }
    }

    fn extract(&mut self, _: crate::ExtractionHandle, content: &mut dyn Content<T>) -> Option<Box<dyn Content<T>>> {
        if self.current_is_folder {
            Some(Box::new(FolderContent::with_content_type(self.entry.path.as_path(), content.content_type()?)))
        } else {
            Some(Box::new(FileContent::with_size(self.entry.path.as_path(), self.entry.size)))
        }
    }

    fn release(&mut self, handle: crate::ExtractionHandle) {
        self.pool.release_slot(handle);
    }
}
