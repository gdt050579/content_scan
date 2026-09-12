use crate::MyTypes;
use content_scan::*;

pub struct ManagedResourceExtractor;

struct ManagedResourceSession {
    content: OwnedContentPtr<MyTypes>,
    remaining: u8,
    index: usize,
    offset: u64,
    current_blob: Vec<u8>,
    entry: Entry,
}

impl ContentExtractor<MyTypes> for ManagedResourceExtractor {
    fn create_session(&mut self, mut content: OwnedContentPtr<MyTypes>, _: &ExtractionContext) -> Option<Box<dyn ExtractionSession<MyTypes>>> {
        // Real PE files are retyped as DotNet when they import mscoree.dll.
        // Their bytes are not the synthetic [len][blob] resource layout.
        if content.size() >= 0x40 {
            if let Some(e_lfanew) = content.read_le_u32(0x3C) {
                let mut sig = [0u8; 4];
                if content.read_into(e_lfanew as u64, 4, &mut sig) == Some(4) && &sig == b"PE\0\0" {
                    return None;
                }
            }
        }
        let remaining = content.read_byte(3)?;
        Some(Box::new(ManagedResourceSession {
            content,
            remaining,
            index: 0,
            offset: 4,
            current_blob: Vec::new(),
            entry: Entry::default(),
        }))
    }
}

impl ExtractionSession<MyTypes> for ManagedResourceSession {
    fn advance(&mut self) -> Option<&Entry> {
        if self.remaining == 0 {
            return None;
        }
        let len = self.content.read_byte(self.offset)? as u64;
        self.offset += 1;
        let mut blob = vec![0u8; len as usize];
        let n = self.content.read_into(self.offset, len as u32, &mut blob)?;
        if n as u64 != len {
            return None;
        }
        self.offset += len;
        self.remaining -= 1;
        self.current_blob = blob;
        let path = format!("res://{}", self.index);
        self.entry.path.set_from_str(&path);
        self.entry.size = self.current_blob.len() as u64;
        self.entry.skip_from_filtering = false;
        self.index += 1;
        Some(&self.entry)
    }
    fn extract(&mut self) -> Option<Box<dyn Content<MyTypes>>> {
        let path = self.entry.path.as_printable_string().to_string();
        Some(Box::new(BufferContent::<MyTypes>::from_parts(
            std::mem::take(&mut self.current_blob),
            path,
            Some(MyTypes::Resource),
        )))
    }
}
