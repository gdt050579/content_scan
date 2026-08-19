use crate::{ContentIdentifier, ContentType};
use std::marker::PhantomData;

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