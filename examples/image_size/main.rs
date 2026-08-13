mod bmp;
mod jpeg;
mod png;

use content_scan::*;
use std::path::Path;

#[derive(Debug, Copy, Clone, Eq, PartialEq, VarMapValue)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, ContentType)]
#[repr(u16)]
pub enum ImageType {
    Png,
    Bmp,
    Jpeg,
    Folder,
}

fn print_results(result: &ScanResult<ImageType>, handle: ScanContentHandle, depth: i32) {
    let path = result.path(handle);
    let content_type = result.content_type(handle);
    for _ in 0..depth {
        print!("  ");
    }
    print!("{} [{:?}]", path.unwrap(), content_type);
    if let Some(size) = result.local(handle).and_then(|v| v.get::<Size>(var!("size"))) {
        println!("  => {} x {}", size.width, size.height);
    } else {
        println!("");
    }
    if let Some(child) = result.child(handle) {
        print_results(result, child, depth + 1);
    }
    if let Some(next) = result.next_sibling(handle) {
        print_results(result, next, depth);
    }
}

fn main() {
    let path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            println!("usage: image_size <file|directory>");
            return;
        }
    };
    let mut scanner = ScannerBuilder::new()
        .filter(
            FilterBuilder::new()
                .include_extensions(Precedence::Medium, &["jpg", "bmp", "png"])
                .deny_the_rest()
                .build(),
        )
        .add_identifier(ImageType::Png, png::PngIdentifier {})
        .add_analyzer(ImageType::Png, 0, png::PngAnalyzer {})
        .add_identifier(ImageType::Bmp, bmp::BmpIdentifier {})
        .add_analyzer(ImageType::Bmp, 0, bmp::BmpAnalyzer {})
        .add_identifier(ImageType::Jpeg, jpeg::JpegIdentifier {})
        .add_analyzer(ImageType::Jpeg, 0, jpeg::JpegAnalyzer {})
        .add_extractor(ImageType::Folder, 0, FolderExtractor::<ImageType>::new(false))
        .build();

    let res = if Path::new(&path).is_dir() {
        let mut content = FolderContent::<ImageType>::with_content_type(&path, ImageType::Folder);
        scanner.scan(&mut content, false)
    } else {
        let mut content = FileContent::<ImageType>::new(&path);
        scanner.scan(&mut content, true)
    };

    println!("Scanned : {} files", res.objects_scanned());
    if res.objects_scanned() > 0 {
        print_results(&res, res.root().unwrap(), 0);
    }
}
