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
    fn analyze(&mut self, content: &mut dyn Content<ImageType>, context: &mut Context<ImageType>) -> NextAction {
        let size = content.size();
        if size < 2 {
            return NextAction::Continue;
        }

        let Some(soi) = content.read(0, 2) else {
            return NextAction::Continue;
        };
        if soi.len() < 2 || soi[0] != 0xFF || soi[1] != 0xD8 {
            return NextAction::Continue;
        }

        let mut i = 2u64;
        while i + 9 < size {
            let Some(b) = content.read(i, 1) else {
                break;
            };
            if b.is_empty() {
                break;
            }
            if b[0] != 0xFF {
                i += 1;
                continue; // skip padding
            }

            let Some(mb) = content.read(i + 1, 1) else {
                break;
            };
            if mb.is_empty() {
                break;
            }
            let marker = mb[0];

            // SOF markers carry the dimensions; exclude DHT/DAC/DRI etc.
            let is_sof = matches!(
                marker,
                0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF
            );
            if is_sof {
                let Some(dim) = content.read(i + 5, 4) else {
                    break;
                };
                if dim.len() < 4 {
                    break;
                }
                let h = u16::from_be_bytes([dim[0], dim[1]]) as u32;
                let w = u16::from_be_bytes([dim[2], dim[3]]);
                context.local().set(
                    var!("size"),
                    Size {
                        width: w as u32,
                        height: h,
                    },
                );
                return NextAction::Continue;
            }

            // standalone markers (RSTn, SOI, EOI, TEM) have no length payload
            if matches!(marker, 0x01 | 0xD0..=0xD9) {
                i += 2;
                continue;
            }

            let Some(lb) = content.read(i + 2, 2) else {
                break;
            };
            if lb.len() < 2 {
                break;
            }
            let len = u16::from_be_bytes([lb[0], lb[1]]) as u64;
            i += 2 + len;
        }

        NextAction::Continue
    }
}
