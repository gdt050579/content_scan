use crate::{
    BufferContent, ContentExtractor, ContentIdentifier, ContentReader, ContentType, Entry, ExtractionContext, ExtractionSession, FileContent,
    OwnedContentPtr,
};
use std::io::Write;
use std::{io::Read, marker::PhantomData};
use zip::ZipArchive;

use std::sync::atomic::{AtomicU64, Ordering};

fn unique_temp_path() -> std::path::PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut p = std::env::temp_dir();
    p.push(format!("content_scan_zip_{}_{}.tmp", std::process::id(), n));
    p
}
/// A [`ContentIdentifier`] for ZIP archives.
///
/// Fast identification is the local-file magic `PK\x03\x04`.
/// [`validate`](ContentIdentifier::validate) then looks for an End of
/// Central Directory record in the tail of the content, so a file
/// that merely starts with those four bytes is rejected. Identification
/// is by content, not by `.zip` extension.
///
/// ```ignore
/// let mut scanner = ScannerBuilder::<MyTypes>::new()
///     .add_identifier(MyTypes::Zip, ZipIdentifier::new())
///     .build();
/// ```
pub struct ZipIdentifier<T: ContentType> {
    _marker: PhantomData<T>,
}
impl<T: ContentType> ZipIdentifier<T> {
    const EOCD_SIG: [u8; 4] = [0x50, 0x4b, 0x05, 0x06];
    /// Creates a new ZIP identifier.
    pub fn new() -> Self {
        Self { _marker: PhantomData }
    }
    fn eocd_tail_ok(buf: &[u8], i: usize, ofs: u64, size: u64) -> bool {
        if i + 22 > buf.len() {
            return false;
        }
        let comment_len = u16::from_le_bytes([buf[i + 20], buf[i + 21]]) as u64;
        (ofs + i as u64) + 22 + comment_len == size
    }
    fn contains_eocd(buf: &[u8], ofs: u64, size: u64) -> bool {
        if buf.len() < 22 {
            return false;
        }
        for i in (0..=buf.len() - 4).rev() {
            if buf[i..i + 4] == Self::EOCD_SIG && Self::eocd_tail_ok(buf, i, ofs, size) {
                return true;
            }
        }
        false
    }
}
impl<T: ContentType> ContentIdentifier<T> for ZipIdentifier<T> {
    fn identify_method(&self) -> Option<crate::IdentifyMethod> {
        Some(crate::IdentifyMethod::Magic(&[0x50, 0x4b, 0x03, 0x04]))
    }

    fn validate(&self, content: &mut dyn crate::Content<T>) -> bool {
        let size = content.size();
        if size < 22 {
            return false;
        }

        let quick_len = size.min(512);
        let quick_start = size - quick_len;
        if let Some(buf) = content.read(quick_start, quick_len as u32) {
            if Self::contains_eocd(buf, quick_start, size) {
                return true;
            }
        }
        let mut buf: [u8; 65557] = [0; 65557];
        let mut pos = size.saturating_sub(65557);
        let region_start = pos;
        let mut index = 0;
        while pos < size {
            let remains = size - pos;
            if let Some(bf) = content.read(pos, remains as u32) {
                buf[index..index + bf.len()].copy_from_slice(bf);
                index += bf.len();
                pos += bf.len() as u64;
                if bf.is_empty() {
                    break;
                }
            } else {
                break;
            }
        }
        Self::contains_eocd(&buf[..index], region_start, size)
    }
}
impl<T: ContentType> Default for ZipIdentifier<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// A [`ContentExtractor`] that unpacks the members of a ZIP archive.
///
/// Pair it with [`ZipIdentifier`] so the scanner both recognizes ZIP
/// files and enumerates their entries. Each regular file becomes a
/// child [`Content`](crate::Content): small members (`< 1 MiB`) are
/// emitted as an in-memory [`BufferContent`], larger ones are
/// decompressed to a temp file and wrapped in [`FileContent`].
/// Directory entries inside the archive are skipped.
///
/// The session reads the parent through a [`ContentReader`], so the
/// source can be a file, a buffer, or any other
/// [`Content`](crate::Content).
///
/// ```ignore
/// let mut scanner = ScannerBuilder::<MyTypes>::new()
///     .add_identifier(MyTypes::Zip, ZipIdentifier::new())
///     .add_extractor(MyTypes::Zip, ZipExtractor::new())
///     .build();
/// ```
pub struct ZipExtractor<T: ContentType> {
    _marker: PhantomData<T>,
}
impl<T: ContentType> ZipExtractor<T> {
    /// Creates a new ZIP extractor.
    pub fn new() -> Self {
        Self { _marker: PhantomData }
    }
}
impl<T: ContentType + 'static> ContentExtractor<T> for ZipExtractor<T> {
    fn create_session(&mut self, content: OwnedContentPtr<T>, _: &ExtractionContext) -> Option<Box<dyn ExtractionSession<T>>> {
        ZipArchive::new(ContentReader::new(content))
            .ok()
            .map(|archive| Box::new(ZipExtractionSession::new(archive)) as Box<dyn ExtractionSession<T>>)
    }
}
impl<T: ContentType> Default for ZipExtractor<T> {
    fn default() -> Self {
        Self::new()
    }
}

struct ZipExtractionSession<T: ContentType> {
    _marker: PhantomData<T>,
    archive: ZipArchive<ContentReader<T>>,
    idx: usize,
    count: usize,
    entry: Entry,
}
impl<T: ContentType> ZipExtractionSession<T> {
    pub fn new(archive: ZipArchive<ContentReader<T>>) -> Self {
        let files_count = archive.len();
        Self {
            _marker: PhantomData,
            archive,
            idx: usize::MAX,
            count: files_count,
            entry: Entry::default(),
        }
    }
}
impl<T: ContentType + 'static> ExtractionSession<T> for ZipExtractionSession<T> {
    fn advance(&mut self) -> Option<&crate::Entry> {
        loop {
            if self.idx == usize::MAX {
                if self.count == 0 {
                    return None;
                }
                self.idx = 0;
            } else {
                if self.idx >= self.count {
                    return None;
                }
                self.idx += 1;
            }

            if let Ok(zip_entry) = self.archive.by_index(self.idx) {
                if zip_entry.is_dir() {
                    continue;
                }
                self.entry.size = zip_entry.size();
                self.entry.skip_from_filtering = false;
                if let Some(p) = zip_entry.enclosed_name() {
                    self.entry.path.set_from_os(p.as_path());
                } else {
                    self.entry.path.set_from_str("");
                }
                return Some(&self.entry);
            }
        }
    }

    fn extract(&mut self) -> Option<Box<dyn crate::Content<T>>> {
        let mut zip_entry = self.archive.by_index(self.idx).ok()?;
        let size = zip_entry.size();

        if size < 0x100000 {
            let mut data = Vec::with_capacity(size as usize);
            zip_entry.read_to_end(&mut data).ok()?;
            Some(Box::new(BufferContent::<T>::from_vec(data, self.entry.path.as_printable_string())))
        } else {
            // Large: decompress to a temp file
            let path = unique_temp_path();
            let mut out = std::fs::File::create(&path).ok()?;
            std::io::copy(&mut zip_entry, &mut out).ok()?;
            out.flush().ok()?;
            drop(out);

            Some(Box::new(FileContent::<T>::with_size(
                &path, size, false, // shared LRU: temp file is short-lived, don't mmap/lock it
            )))
        }
    }
}
