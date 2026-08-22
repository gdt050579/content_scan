use content_scan::*;
use std::path::Path;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ContentType)]
#[repr(u16)]
enum MyTypes {
    Folder,
}

/// Shannon entropy in bits per byte, attached to each finding.
#[derive(Copy, Clone, Debug)]
struct Entropy(f64);
impl FindingMetadata for Entropy {}

fn shannon_entropy(counts: &[u64; 256], total: u64) -> f64 {
    if total == 0 {
        return 0.0;
    }
    let total = total as f64;
    let mut entropy = 0.0;
    for &count in counts {
        if count > 0 {
            let p = count as f64 / total;
            entropy -= p * p.log2();
        }
    }
    entropy
}

#[derive(Dependencies)]
#[Dependencies(name = "EntropyAnalyzer")]
struct EntropyAnalyzer;
impl ContentAnalyzer<MyTypes, Entropy> for EntropyAnalyzer {
    fn analyze(&mut self, content: &mut dyn Content<MyTypes>, context: &mut Context<MyTypes, Entropy>) -> NextAction {
        if content.content_type() == Some(MyTypes::Folder) {
            return NextAction::Continue;
        }
        let size = content.size();
        let mut counts = [0u64; 256];
        let mut offset = 0u64;
        let mut total = 0u64;
        while offset < size {
            let to_read = (size - offset).min(0x100000u64) as u32;
            match content.read(offset, to_read) {
                Some(buf) if !buf.is_empty() => {
                    for &b in buf {
                        counts[b as usize] += 1;
                    }
                    total += buf.len() as u64;
                    offset += buf.len() as u64;
                }
                _ => break,
            }
        }
        let entropy = shannon_entropy(&counts, total);
        let label = if entropy > 7.8 {
            "packed"
        } else if entropy > 7.0 {
            "encrypted"
        } else {
            "normal"
        };
        context.add_finding(label, None, Some(Entropy(entropy)));
        NextAction::Continue
    }
}

fn main() {
    let path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            println!("usage: entropy <file|directory>");
            return;
        }
    };
    let mut scanner = ScannerBuilder::with_metadata::<Entropy>()
        .max_depth(64)
        .add_generic_analyzer(0, EntropyAnalyzer {})
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
        let entropy = f.metadata().map(|m| m.0).unwrap_or(0.0);
        println!("{:<10}  {:>6.4}  {}", f.finding(), entropy, f.path().unwrap_or_default());
    }
}
