use content_scan::*;
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ContentType)]
#[repr(u16)]
enum MyTypes {
    Folder,
}

#[derive(Dependencies)]
#[Dependencies(name = "FindTextAnalyzer")]
struct FindTextAnalyzer {
    needle: Vec<u8>,
}
impl FindTextAnalyzer {
    fn search(&self, content: &mut dyn Content<MyTypes>, context: &mut Context<MyTypes>) {
        let n = self.needle.len();
        if n == 0 {
            return;
        }
        let size = content.size();
        let mut window = Vec::new();
        let mut file_pos = 0u64;
        while file_pos < size {
            let to_read = (size - file_pos).min(0x100000u64) as u32;
            match content.read(file_pos, to_read) {
                Some(buf) if !buf.is_empty() => {
                    let window_start = file_pos - window.len() as u64;
                    window.extend_from_slice(buf);
                    let mut i = 0;
                    while i + n <= window.len() {
                        if window[i..i + n] == self.needle {
                            context.add_finding(&format!("offset {}", window_start + i as u64), None, None);
                            i += n;
                        } else {
                            i += 1;
                        }
                    }
                    let keep = n - 1;
                    if window.len() > keep {
                        window.drain(..window.len() - keep);
                    }
                    file_pos += buf.len() as u64;
                }
                _ => break,
            }
        }
    }
}

impl ContentAnalyzer<MyTypes> for FindTextAnalyzer {
    fn analyze(&mut self, content: &mut dyn Content<MyTypes>, context: &mut Context<MyTypes>) -> NextAction {
        if content.content_type() == Some(MyTypes::Folder) {
            return NextAction::Continue;
        }
        self.search(content, context);
        NextAction::Continue
    }
}

struct Finder {
    started: Option<Instant>,
}
impl ScanObserver<MyTypes> for Finder {
    fn on_begin(&mut self, root: &str) {
        self.started = Some(Instant::now());
        println!("begin    {root}");
    }

    fn on_scan_object(&mut self, path: &str, ty: Option<MyTypes>) {
        println!("scan     {path}  {ty:?}");
    }

    fn on_filtered(&mut self, path: &str) {
        println!("skip     {path}");
    }

    fn on_finding(&mut self, path: &str, finding: &str, _source: Option<&str>, _metadata: Option<&NoMetadata>) {
        println!("find     {path}  {finding}");
    }

    fn on_extraction(&mut self, parent: &str, entry: &Entry) {
        println!("extract  {}  from {parent}  ({} bytes)", entry.path.as_printable_string(), entry.size);
    }

    fn on_end(&mut self) {
        let elapsed = self.started.map(|t| t.elapsed()).unwrap_or_default();
        println!("end      {elapsed:.3?}");
    }
}

fn leak_extensions(spec: &str) -> Vec<&'static str> {
    spec.split(',')
        .map(|s| s.trim().trim_start_matches('.'))
        .filter(|s| !s.is_empty())
        .map(|s| &*Box::leak(s.to_string().into_boxed_str()))
        .collect()
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (path, text, ext_spec) = match (args.next(), args.next()) {
        (Some(path), Some(text)) => (path, text, args.next()),
        _ => {
            println!("usage: finder <file|directory> <text> [ext,ext,...]");
            return;
        }
    };
    if text.is_empty() {
        println!("search text must not be empty");
        return;
    }

    let mut builder = ScannerBuilder::new()
        .max_depth(64)
        .store_findings(false)
        .observer(Finder { started: None })
        .add_generic_analyzer(0, FindTextAnalyzer { needle: text.into_bytes() })
        .add_extractor(MyTypes::Folder, FolderExtractor::<MyTypes>::new(true, false));

    let is_dir = Path::new(&path).is_dir();
    if is_dir {
        if let Some(spec) = ext_spec.as_deref() {
            let exts = leak_extensions(spec);
            if !exts.is_empty() {
                builder = builder.filter(FilterBuilder::new().include_extensions(Precedence::Medium, &exts).deny_the_rest().build());
            }
        }
    }

    let mut scanner = builder.build();
    if is_dir {
        let mut content = FolderContent::<MyTypes>::with_content_type(&path, MyTypes::Folder);
        scanner.scan(&mut content, false);
    } else {
        let mut content = FileContent::<MyTypes>::new(&path, false);
        scanner.scan(&mut content, true);
    }
}
