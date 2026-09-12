mod dotnet_analyzer;
mod dotnet_extractor;
mod pe_analyzer;
mod pe_identifier;

use content_scan::*;
use dotnet_analyzer::DotNetAnalyzer;
use dotnet_extractor::ManagedResourceExtractor;
use pe_analyzer::{DotNetDetector, PeHeaderAnalyzer};
use pe_identifier::PeIdentifier;
use std::path::Path;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ContentType)]
#[repr(u16)]
pub enum MyTypes {
    Pe,
    DotNet,
    Resource,
    Folder,
}

struct Progress;
impl ScanObserver<MyTypes> for Progress {
    fn on_scan_object(&mut self, path: &str, ty: Option<MyTypes>) {
        if ty == Some(MyTypes::Folder) {
            return;
        }
        println!("scanning {path}");
    }
}

fn build_scanner() -> Scanner<MyTypes, NoMetadata> {
    ScannerBuilder::<MyTypes>::new()
        .max_depth(64)
        .observer(Progress)
        .filter(
            FilterBuilder::new()
                .include_extensions(Precedence::Medium, &["exe", "dll"])
                .deny_the_rest()
                .build(),
        )
        .add_identifier(MyTypes::Pe, PeIdentifier)
        .add_analyzer(MyTypes::Pe, 0, PeHeaderAnalyzer)
        .add_analyzer(MyTypes::Pe, 10, DotNetDetector)
        .add_analyzer(MyTypes::DotNet, 0, DotNetAnalyzer)
        .add_extractor(MyTypes::DotNet, ManagedResourceExtractor)
        .add_extractor(MyTypes::Folder, FolderExtractor::<MyTypes>::new(true, false))
        .max_change_type(2)
        .build()
}

fn dump_tree(res: &ScanResult<MyTypes>, handle: ScanContentHandle, depth: usize) {
    let pad = "  ".repeat(depth);
    let path = res.path(handle).unwrap_or("?");
    let ty = res.content_type(handle);
    print!("{pad}- {path} ({ty:?})");
    if let Some(imports) = res.local(handle).and_then(|m| m.get::<&str>(var!("imports"))) {
        if !imports.is_empty() {
            println!("  imports=[{}]", imports.replace('\n', ", "));
        } else {
            println!();
        }
    } else {
        println!();
    }
    let mut child = res.child(handle);
    while let Some(h) = child {
        dump_tree(res, h, depth + 1);
        child = res.next_sibling(h);
    }
}

fn print_scan(res: &ScanResult<MyTypes>) {
    println!("scanned {} objects", res.objects_scanned());
    if let Some(root) = res.root() {
        dump_tree(res, root, 0);
    }
    println!("findings:");
    for f in res.findings() {
        println!(
            "  {}  source={:?}  path={}  type={:?}",
            f.finding(),
            f.source(),
            f.path().unwrap_or("?"),
            f.content_type()
        );
    }
}

fn main() {
    let path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            println!("usage: pe_reprocess <file|directory>");
            return;
        }
    };

    let mut scanner = build_scanner();
    let res = if Path::new(&path).is_dir() {
        let mut content = FolderContent::<MyTypes>::with_content_type(&path, MyTypes::Folder);
        scanner.scan(&mut content, false)
    } else {
        let mut content = FileContent::<MyTypes>::new(&path, false);
        scanner.scan(&mut content, true)
    };
    print_scan(&res);
}
