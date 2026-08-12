mod bmp;
mod jpeg;
mod png;

use std::path::Path;

use content_scan::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq, ContentType)]
#[repr(u16)]
pub enum ImageType {
    Png,
    Bmp,
    Jpeg,
    Folder,
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
        .add_identifier(ImageType::Png, png::PngIdentifier {})
        .add_analyzer(ImageType::Png, 0, png::PngAnalyzer {})
        .add_identifier(ImageType::Bmp, bmp::BmpIdentifier {})
        .add_analyzer(ImageType::Bmp, 0, bmp::BmpAnalyzer {})
        .add_identifier(ImageType::Jpeg, jpeg::JpegIdentifier {})
        .add_analyzer(ImageType::Jpeg, 0, jpeg::JpegAnalyzer {})
        .add_extractor(ImageType::Folder, 0, FolderExtractor::<ImageType>::new(false))
        .build();

    let res = if  Path::new(&path).is_dir() {
        let mut content = FolderContent::<ImageType>::with_content_type(&path, ImageType::Folder);
        scanner.scan(&mut content)
    } else {
        let mut content = FileContent::<ImageType>::new(&path);
        scanner.scan(&mut content)
    };


    println!("Scanned : {} files", res.objects_scanned());
    println!("Type    : {:?}", res.content_type(res.root().unwrap()));
    println!("Path    : {:?}", res.path(res.root().unwrap()));
    match (
        res.global().get::<u32>(var!("width")),
        res.global().get::<u32>(var!("height")),
    ) {
        (Some(w), Some(h)) => println!("{w}x{h}"),
        _ => {
            println!("failed to determine image size for '{path}'");
        }
    }
}
