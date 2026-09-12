use crate::MyTypes;
use content_scan::*;

pub struct PeIdentifier;

impl ContentIdentifier<MyTypes> for PeIdentifier {
    fn identify_method(&self) -> Option<IdentifyMethod> {
        Some(IdentifyMethod::Magic(b"MZ"))
    }

    fn validate(&self, content: &mut dyn Content<MyTypes>) -> bool {
        let Some(e_lfanew) = content.read_le_u32(0x3C) else {
            return false;
        };
        if !matches!(content.read_exact(e_lfanew as u64, 4), Some(b"PE\0\0")) {
            return false;
        }
        // COFF NumberOfSections sits 6 bytes after the PE signature.
        matches!(content.read_le_u16(e_lfanew as u64 + 6), Some(n) if n > 0)
    }
}
