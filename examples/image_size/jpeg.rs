use crate::ImageType;
use crate::Size;
use content_scan::*;

pub struct JpegIdentifier;
impl ContentIdentifier<ImageType> for JpegIdentifier {
    fn identify_method(&self) -> Option<IdentifyMethod> {
        Some(IdentifyMethod::Magic(b"\xFF\xD8"))
    }

    fn validate(&self, content: &mut dyn Content<ImageType>) -> bool {
        content.size() >= 2
    }
}

#[derive(Dependencies)]
#[Dependencies(name = "JpegAnalyzer")]
pub struct JpegAnalyzer;
impl ContentAnalyzer<ImageType> for JpegAnalyzer {
    fn analyze(&mut self, content: &mut dyn Content<ImageType>, context: &mut Context<ImageType>) -> AnalysisOutcome<ImageType> {
        let size = content.size();
        let Some(soi) = content.read_be_u16(0) else {
            return AnalysisOutcome::Continue;
        };
        if soi != 0xFFD8 {
            return AnalysisOutcome::Continue;
        }

        let mut i = 2u64;
        while i + 9 < size {
            let Some(b) = content.read_byte(i) else {
                break;
            };
            if b != 0xFF {
                i += 1;
                continue; // skip padding
            }

            let Some(marker) = content.read_byte(i + 1) else {
                break;
            };
            // SOF markers carry the dimensions; exclude DHT/DAC/DRI etc.
            let is_sof = matches!(
                marker,
                0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF
            );
            if is_sof {
                let Some(h) = content.read_be_u16(i + 5) else {
                    break;
                };
                let Some(w) = content.read_be_u16(i + 7) else {
                    break;
                };
                context.local().set(
                    var!("size"),
                    Size {
                        width: w as u32,
                        height: h as u32,
                    },
                );
                return AnalysisOutcome::Continue;
            }

            // standalone markers (RSTn, SOI, EOI, TEM) have no length payload
            if matches!(marker, 0x01 | 0xD0..=0xD9) {
                i += 2;
                continue;
            }

            let Some(lb) = content.read_be_u16(i + 2) else {
                break;
            };
            i += 2 + lb as u64;
        }

        AnalysisOutcome::Continue
    }
}
