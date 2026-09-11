use crate::ImageType;
use crate::Size;
use content_scan::*;

pub struct PngIdentifier;
impl ContentIdentifier<ImageType> for PngIdentifier {
    fn identify_method(&self) -> Option<IdentifyMethod> {
        Some(IdentifyMethod::Magic(b"\x89PNG\r\n\x1a\n"))
    }

    fn validate(&self, content: &mut dyn Content<ImageType>) -> bool {
        content.size() >= 24
    }
}

#[derive(Dependencies)]
#[Dependencies(name = "PngAnalyzer")]
pub struct PngAnalyzer;
impl ContentAnalyzer<ImageType> for PngAnalyzer {
    fn analyze(&mut self, content: &mut dyn Content<ImageType>, context: &mut Context<ImageType>) -> AnalysisOutcome<ImageType> {
        let Some(d) = content.read_exact(0, 24) else {
            return AnalysisOutcome::Continue;
        };
        let w = u32::from_be_bytes(d[16..20].try_into().unwrap());
        let h = u32::from_be_bytes(d[20..24].try_into().unwrap());
        context.local().set(var!("size"), Size { width: w, height: h });
        AnalysisOutcome::Continue
    }
}
