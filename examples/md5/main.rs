use content_scan::*;
use md5::{Digest, Md5};
use std::path::Path;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ContentType)]
#[repr(u16)]
enum MyTypes {
    Folder,
}

#[derive(Dependencies)]
#[Dependencies(name = "ComputeHashAnalyzer")]
struct ComputeHashAnalyzer;
impl ContentAnalyzer<MyTypes> for ComputeHashAnalyzer {
    fn analyze(&mut self, content: &mut dyn Content<MyTypes>, context: &mut Context<MyTypes>) -> NextAction {
        if content.content_type() == Some(MyTypes::Folder) {
            return NextAction::Continue;
        }
        let size = content.size();
        let mut hasher = Md5::new();
        let mut offset = 0u64;
        while offset < size {
            let to_read = (size - offset).min(0x100000u64) as u32;
            match content.read(offset, to_read) {
                Some(buf) if !buf.is_empty() => {
                    hasher.update(buf);
                    offset += buf.len() as u64;
                }
                _ => break,
            }
        }
        context.add_finding(format!("{:x}", hasher.finalize()).as_str(), None, None);
        NextAction::Continue
    }
}

fn main() {
    let path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            println!("usage: md5 <file|directory>");
            return;
        }
    };
    let mut scanner = ScannerBuilder::new()
        .max_depth(64)
        .add_generic_analyzer(0, ComputeHashAnalyzer {})
        .add_extractor(MyTypes::Folder, FolderExtractor::<MyTypes>::new(true, false))
        .build();

    let res = if Path::new(&path).is_dir() {
        let mut content = FolderContent::<MyTypes>::with_content_type(&path, MyTypes::Folder);
        scanner.scan(&mut content, false)
    } else {
        let mut content = FileContent::<MyTypes>::new(&path, false);
        scanner.scan(&mut content, true)
    };
    println!("scanned {} objects", res.objects_scanned());
    for f in res.findings() {
        println!("{}  {}", f.finding(), f.path().unwrap_or_default());
    }
}
