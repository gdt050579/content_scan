use content_scan::*;
use std::path::Path;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ContentType)]
#[repr(u16)]
enum MyTypes {
    Text,
    Base64,
    Base64Decoded,
    Folder,
}

const MIN_BASE64_LEN: u64 = 16;

fn is_base64_alphabet(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'+' || b == b'/'
}

fn read_range(content: &mut dyn Content<MyTypes>, offset: u64, length: u64) -> Option<Vec<u8>> {
    let mut buf = Vec::with_capacity(length as usize);
    let end = offset.saturating_add(length);
    let mut off = offset;
    while off < end {
        let want = (end - off).min(u32::MAX as u64) as u32;
        let chunk = content.read(off, want)?;
        if chunk.is_empty() {
            break;
        }
        let take = (end - off).min(chunk.len() as u64) as usize;
        buf.extend_from_slice(&chunk[..take]);
        off += take as u64;
    }
    if buf.len() as u64 == length {
        Some(buf)
    } else {
        None
    }
}

fn decode_base64(input: &[u8]) -> Option<Vec<u8>> {
    if input.is_empty() || !input.len().is_multiple_of(4) {
        return None;
    }
    let value = |c: u8| -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    };
    let mut out = Vec::with_capacity(input.len() / 4 * 3);
    for chunk in input.chunks_exact(4) {
        let mut pads = 0u8;
        let mut n = 0u32;
        for (i, &c) in chunk.iter().enumerate() {
            if c == b'=' {
                if i < 2 {
                    return None;
                }
                pads += 1;
            } else if pads > 0 {
                return None;
            } else {
                n |= (value(c)? as u32) << (18 - 6 * i);
            }
        }
        out.push((n >> 16) as u8);
        if pads < 2 {
            out.push((n >> 8) as u8);
        }
        if pads < 1 {
            out.push(n as u8);
        }
    }
    Some(out)
}

struct TextIdentifier;
impl ContentIdentifier<MyTypes> for TextIdentifier {
    fn identify_method(&self) -> Option<IdentifyMethod> {
        None
    }
    fn validate(&self, _: &mut dyn Content<MyTypes>) -> bool {
        true
    }
}

/// Walks a file looking for contiguous Base64 runs and queues a `Base64` extraction for each one.
#[derive(Dependencies)]
#[Dependencies(name = "Base64Finder")]
struct Base64Finder;
impl ContentAnalyzer<MyTypes> for Base64Finder {
    fn analyze(&mut self, content: &mut dyn Content<MyTypes>, context: &mut Context<MyTypes>) -> NextAction {
        let size = content.size();
        let mut pos = 0u64;
        while pos < size {
            let Some(b) = content.read(pos, 1).and_then(|s| s.first().copied()) else {
                break;
            };
            if !is_base64_alphabet(b) {
                pos += 1;
                continue;
            }
            let start = pos;
            pos += 1;
            while pos < size {
                match content.read(pos, 1).and_then(|s| s.first().copied()) {
                    Some(c) if is_base64_alphabet(c) => pos += 1,
                    _ => break,
                }
            }
            let mut pads = 0u64;
            while pads < 2 && pos + pads < size {
                match content.read(pos + pads, 1).and_then(|s| s.first().copied()) {
                    Some(b'=') => pads += 1,
                    _ => break,
                }
            }
            let len = pos - start + pads;
            if len >= MIN_BASE64_LEN && len.is_multiple_of(4) {
                context.request_extract(MyTypes::Base64).at(start).len(len).emit();
            }
            pos += pads;
        }
        NextAction::Continue
    }
}

/// Reads the requested slice, Base64-decodes it into a `Vec<u8>`, and yields it as `Base64Decoded`.
#[derive(Dependencies)]
#[Dependencies(name = "Base64Extractor")]
struct Base64Extractor;

struct Base64Session {
    content: OwnedContentPtr<MyTypes>,
    offset: u64,
    length: u64,
    done: bool,
    entry: Entry,
}

impl ContentExtractor<MyTypes> for Base64Extractor {
    fn create_session(
        &mut self,
        content: OwnedContentPtr<MyTypes>,
        extract_context: &ExtractionContext,
    ) -> Option<Box<dyn ExtractionSession<MyTypes>>> {
        let length = extract_context.length.unwrap_or(content.size().saturating_sub(extract_context.offset));
        if length == 0 {
            return None;
        }
        Some(Box::new(Base64Session {
            content,
            offset: extract_context.offset,
            length,
            done: false,
            entry: Entry::default(),
        }))
    }
}

impl ExtractionSession<MyTypes> for Base64Session {
    fn advance(&mut self) -> Option<&Entry> {
        if self.done {
            return None;
        }
        self.done = true;
        let path = format!("base64@{}", self.offset);
        self.entry.path.set_from_str(&path);
        self.entry.size = self.length / 4 * 3;
        self.entry.skip_from_filtering = false;
        Some(&self.entry)
    }
    fn extract(&mut self) -> Option<Box<dyn Content<MyTypes>>> {
        let encoded = read_range(&mut *self.content, self.offset, self.length)?;
        let decoded = decode_base64(&encoded)?;
        Some(Box::new(BufferContent::<MyTypes>::from_parts(
            decoded,
            format!("base64@{}", self.offset),
            Some(MyTypes::Base64Decoded),
        )))
    }
}

#[derive(Dependencies)]
#[Dependencies(name = "Base64DecodedAnalyzer")]
struct Base64DecodedAnalyzer;
impl ContentAnalyzer<MyTypes> for Base64DecodedAnalyzer {
    fn analyze(&mut self, content: &mut dyn Content<MyTypes>, _: &mut Context<MyTypes>) -> NextAction {
        let path = content.path().as_printable_string().to_string();
        let n = content.size().min(u32::MAX as u64) as u32;
        let buf = content.read(0, n).unwrap_or(&[]);
        println!("{}: {}", path, String::from_utf8_lossy(buf));
        NextAction::Continue
    }
}

fn build_scanner() -> Scanner<MyTypes, NoMetadata> {
    ScannerBuilder::new()
        .add_identifier(MyTypes::Text, TextIdentifier {})
        .add_analyzer(MyTypes::Text, 0, Base64Finder {})
        .add_extractor(MyTypes::Base64, Base64Extractor)
        .add_analyzer(MyTypes::Base64Decoded, 0, Base64DecodedAnalyzer {})
        .add_extractor(MyTypes::Folder, FolderExtractor::<MyTypes>::new(true, false))
        .build()
}

const SAMPLE: &[u8] = b"A short note.\n\
encoded: SGVsbG8sIHdvcmxkIQ==\n\
and also: VGhpcyBpcyBiYXNlNjQu\n\
tag: Y29udGVudF9zY2Fu\n";

fn main() {
    let mut scanner = build_scanner();
    let path = std::env::args().nth(1);
    let res = match path.as_deref() {
        None => {
            let mut content = BufferContent::<MyTypes>::new(SAMPLE, "sample.txt");
            scanner.scan(&mut content, true)
        }
        Some(p) if Path::new(p).is_dir() => {
            let mut content = FolderContent::<MyTypes>::with_content_type(p, MyTypes::Folder);
            scanner.scan(&mut content, false)
        }
        Some(p) => {
            let mut content = FileContent::<MyTypes>::new(p, false);
            scanner.scan(&mut content, true)
        }
    };
    println!("scanned {} objects", res.objects_scanned());
}
