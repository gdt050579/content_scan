mod bmp;
mod png;

use content_scan::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq, ContentType)]
#[repr(u16)]
pub enum ImageType {
    Png,
    Bmp,
}

fn main() {
    let path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            println!("usage: image_size <file>");
            return;
        }
    };
    let mut scanner = ScannerBuilder::new()
        .add_identifier(ImageType::Png, png::PngIdentifier {})
        .add_analyzer(ImageType::Png, 0, png::PngAnalyzer {})
        .add_identifier(ImageType::Bmp, bmp::BmpIdentifier {})
        .add_analyzer(ImageType::Bmp, 0, bmp::BmpAnalyzer {})
        .build();

    let mut content = FileContent::<ImageType>::new(&path);
    let res = scanner.scan(&mut content);

    println!("Scanned: {} files", res.objects_scanned());
    println!("Type   : {:?}", res.content_type(res.root().unwrap()));
    println!("Path   : {:?}", res.path(res.root().unwrap()));
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
