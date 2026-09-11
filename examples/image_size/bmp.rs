use crate::ImageType;
use crate::Size;
use content_scan::*;

pub struct BmpIdentifier;
impl ContentIdentifier<ImageType> for BmpIdentifier {
    fn identify_method(&self) -> Option<IdentifyMethod> {
        Some(IdentifyMethod::Magic(b"BM"))
    }

    fn validate(&self, content: &mut dyn Content<ImageType>) -> bool {
        content.size() >= 26
    }
}

#[derive(Dependencies)]
#[Dependencies(name = "BmpAnalyzer")]
pub struct BmpAnalyzer;
impl ContentAnalyzer<ImageType> for BmpAnalyzer {
    fn analyze(&mut self, content: &mut dyn Content<ImageType>, context: &mut Context<ImageType>) -> AnalysisOutcome {
        let Some(d) = content.read_exact(0, 26) else {
            return AnalysisOutcome::Continue;
        };
        let w = u32::from_le_bytes(d[18..22].try_into().unwrap());
        let h = u32::from_le_bytes(d[22..26].try_into().unwrap());
        context.local().set(var!("size"), Size { width: w, height: h });
        AnalysisOutcome::Continue
    }
}
