use content_scan::*;
use crate::Size;
use crate::ImageType;

pub struct PngIdentifier;
impl ContentIdentifier<ImageType> for PngIdentifier {
    fn identify_method(&self) -> Option<IdentifyMethod> {
        Some(IdentifyMethod::Magic(b"\x89PNG\r\n\x1a\n"))
    }

    fn validate(&self, content: &mut dyn Content<ImageType>) -> bool {
        content.size() >= 24    
    }
}

pub struct PngAnalyzer;
impl ContentAnalyzer<ImageType> for PngAnalyzer {
    fn analyze(&mut self, content: &mut dyn Content<ImageType>, context: &mut Context<ImageType>) -> NextAction {
        let Some(d) = content.read(0, 24) else {
            return NextAction::Continue;
        };
        if d.len() < 24 {
            return NextAction::Continue;
        }
        let w = u32::from_be_bytes(d[16..20].try_into().unwrap());
        let h = u32::from_be_bytes(d[20..24].try_into().unwrap());
        context.local().set(var!("size"), Size { width: w, height: h });
        NextAction::Continue
    }
}
