use content_scan::*;
use std::path::Path;

#[derive(Debug, Copy, Clone, Eq, PartialEq, VarMapValue)]
struct Size {
    width: u32,
    height: u32,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ContentType)]
#[repr(u16)]
enum MyTypes {
    Zip,
    Png,
}

struct PngIdentifier;
impl ContentIdentifier<MyTypes> for PngIdentifier {
    fn identify_method(&self) -> Option<IdentifyMethod> {
        Some(IdentifyMethod::Magic(b"\x89PNG\r\n\x1a\n"))
    }

    fn validate(&self, content: &mut dyn Content<MyTypes>) -> bool {
        content.size() >= 24
    }
}

#[derive(Dependencies)]
#[Dependencies(name = "PngAnalyzer")]
struct PngAnalyzer;
impl ContentAnalyzer<MyTypes> for PngAnalyzer {
    fn analyze(&mut self, content: &mut dyn Content<MyTypes>, context: &mut Context<MyTypes>) -> NextAction {
        context.local().set(var!("file_size"), content.size());
        let Some(d) = content.read(0, 24) else {
            return NextAction::Continue;
        };
        if d.len() < 24 {
            return NextAction::Continue;
        }
        let width = u32::from_be_bytes(d[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(d[20..24].try_into().unwrap());
        context.local().set(var!("size"), Size { width, height });
        NextAction::Continue
    }
}

fn print_pngs(result: &ScanResult<MyTypes>, handle: ScanContentHandle) {
    if result.content_type(handle) == Some(MyTypes::Png) {
        let path = result.path(handle).unwrap_or("<unknown>");
        let file_size = result.local(handle).and_then(|v| v.get::<u64>(var!("file_size"))).unwrap_or(0);
        if let Some(size) = result.local(handle).and_then(|v| v.get::<Size>(var!("size"))) {
            println!("{}  => {} x {}  ({} bytes)", path, size.width, size.height, file_size);
        } else {
            println!("{}  => {} bytes", path, file_size);
        }
    }
    if let Some(child) = result.child(handle) {
        print_pngs(result, child);
    }
    if let Some(next) = result.next_sibling(handle) {
        print_pngs(result, next);
    }
}

fn main() {
    let path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            println!("usage: zip_png_size <archive.zip>");
            return;
        }
    };
    if !Path::new(&path).is_file() {
        println!("not a file: {path}");
        return;
    }

    let mut scanner = ScannerBuilder::new()
        .max_depth(64)
        .filter(
            FilterBuilder::new()
                .include_extensions(Precedence::Medium, &["png"])
                .deny_the_rest()
                .build(),
        )
        .add_identifier(MyTypes::Zip, ZipIdentifier::new())
        .add_extractor(MyTypes::Zip, ZipExtractor::new())
        .add_identifier(MyTypes::Png, PngIdentifier)
        .add_analyzer(MyTypes::Png, 0, PngAnalyzer)
        .build();

    let mut content = FileContent::<MyTypes>::new(&path, false);
    // `false`: do not filter the ZIP itself, only its extracted entries
    let res = scanner.scan(&mut content, false);

    println!("Scanned: {} objects", res.objects_scanned());
    if let Some(root) = res.root() {
        if res.content_type(root) != Some(MyTypes::Zip) {
            println!("not a ZIP archive: {path}");
            return;
        }
        print_pngs(&res, root);
    }
}
