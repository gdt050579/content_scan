use content_scan::*;

use crate::ImageType;

pub struct PngIdentifier;
impl ContentIdentifier<ImageType> for PngIdentifier {
    fn identify_method(&self) -> Option<IdentifyMethod> {
        Some(IdentifyMethod::Magic(b"\x89PNG\r\n\x1a\n"))
    }

    fn validate(&self, content: &dyn Content<ImageType>) -> bool {
        content.size() >= 24
    }
}

pub struct PngAnalyzer;
impl ContentAnalyzer<ImageType> for PngAnalyzer {
    fn analyze(&mut self, content: &mut dyn Content<ImageType>, context: &mut Context) -> NextAction {
        let Some(d) = content.read(0, 24) else {
            return NextAction::Continue;
        };
        if d.len() < 24 {
            return NextAction::Continue;
        }
        let w = u32::from_le_bytes(d[16..20].try_into().unwrap());
        let h = u32::from_le_bytes(d[20..24].try_into().unwrap());
        context.global().set(var!("width"), w);
        context.global().set(var!("height"), h);
        NextAction::Continue
    }
}
