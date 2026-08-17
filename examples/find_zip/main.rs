use content_scan::*;
use std::path::Path;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ContentType)]
#[repr(u16)]
enum MyTypes {
    Zip,
    Folder,
}

struct ZipPrinter;
impl ContentAnalyzer<MyTypes> for ZipPrinter {
    fn analyze(&mut self, content: &mut dyn Content<MyTypes>, _: &mut Context<MyTypes>) -> NextAction {
        println!("{}", content.path().as_printable_string());
        NextAction::Continue
    }
}

fn main() {
    let path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            println!("usage: find_zip <file|directory>");
            return;
        }
    };
    let mut scanner = ScannerBuilder::new()
        .max_depth(64)
        .add_identifier(MyTypes::Zip, ZipIdentifier::new())
        .add_analyzer(MyTypes::Zip, 0, ZipPrinter {})
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
}
