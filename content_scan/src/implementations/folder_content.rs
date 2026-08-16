use crate::{Content, ContentType, ContentPath};
use std::path::Path;

/// A [`Content`] standing for a directory rather than a byte stream.
///
/// It carries only a path: [`size`](Content::size) is always `0` and
/// [`read`](Content::read) always returns `None`. Its purpose is to
/// give the scanner an object it can dispatch on, so that a
/// [`FolderExtractor`](crate::FolderExtractor) registered for the same content type can
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
    path: ContentPath,
    content_type: T,
}
impl<T: ContentType> FolderContent<T> {
    /// Creates a `FolderContent` for `path`, tagged as `content_type`.
    ///
    /// The path is stored via [`ContentPath::from_os`].
    pub fn with_content_type(path: impl AsRef<Path>, content_type: T) -> Self {
        Self {
            path: ContentPath::from_os(path.as_ref()),
            content_type,
        }
    }
}
impl<T: ContentType> Content<T> for FolderContent<T> {
    fn path(&self) -> &ContentPath {
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
