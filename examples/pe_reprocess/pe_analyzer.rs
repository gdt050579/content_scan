use crate::MyTypes;
use content_scan::*;

const PE32_MAGIC: u16 = 0x10B;
const PE32PLUS_MAGIC: u16 = 0x20B;
const IMPORT_DIR_INDEX: u64 = 1;
const MAX_SECTIONS: u16 = 96;
const MAX_IMPORTS: usize = 512;
const MAX_DLL_NAME: u32 = 256;

struct Section {
    virtual_address: u32,
    virtual_size: u32,
    pointer_to_raw_data: u32,
    size_of_raw_data: u32,
}

fn read_u16(content: &mut dyn Content<MyTypes>, offset: u64) -> Option<u16> {
    let mut buf = [0u8; 2];
    (content.read_into(offset, 2, &mut buf)? == 2).then(|| u16::from_le_bytes(buf))
}

fn read_u32(content: &mut dyn Content<MyTypes>, offset: u64) -> Option<u32> {
    let mut buf = [0u8; 4];
    (content.read_into(offset, 4, &mut buf)? == 4).then(|| u32::from_le_bytes(buf))
}

fn machine_name(machine: u16) -> &'static str {
    match machine {
        0x014C => "x86",
        0x8664 => "x64",
        0xAA64 => "arm64",
        0x01C4 => "arm",
        0x0200 => "ia64",
        _ => "unknown",
    }
}

fn rva_to_offset(rva: u32, sections: &[Section]) -> Option<u64> {
    for s in sections {
        let span = s.virtual_size.max(s.size_of_raw_data);
        if rva >= s.virtual_address && rva.wrapping_sub(s.virtual_address) < span {
            return Some(s.pointer_to_raw_data as u64 + (rva - s.virtual_address) as u64);
        }
    }
    None
}

fn read_ascii_z(content: &mut dyn Content<MyTypes>, offset: u64) -> Option<String> {
    let mut buf = vec![0u8; MAX_DLL_NAME as usize];
    let n = content.read_into(offset, MAX_DLL_NAME, &mut buf)?;
    let end = buf[..n].iter().position(|&b| b == 0).unwrap_or(n);
    if end == 0 {
        return None;
    }
    std::str::from_utf8(&buf[..end]).ok().map(|s| s.to_string())
}

fn read_sections(content: &mut dyn Content<MyTypes>, first: u64, count: u16) -> Option<Vec<Section>> {
    let count = count.min(MAX_SECTIONS);
    let mut sections = Vec::with_capacity(count as usize);
    for i in 0..count {
        let off = first + i as u64 * 40;
        sections.push(Section {
            virtual_size: read_u32(content, off + 8)?,
            virtual_address: read_u32(content, off + 12)?,
            size_of_raw_data: read_u32(content, off + 16)?,
            pointer_to_raw_data: read_u32(content, off + 20)?,
        });
    }
    Some(sections)
}

fn read_imports(content: &mut dyn Content<MyTypes>, import_rva: u32, sections: &[Section]) -> Vec<String> {
    let Some(mut offset) = rva_to_offset(import_rva, sections) else {
        return Vec::new();
    };
    let mut imports = Vec::new();
    for _ in 0..MAX_IMPORTS {
        let mut desc = [0u8; 20];
        if content.read_into(offset, 20, &mut desc) != Some(20) || desc.iter().all(|&b| b == 0) {
            break;
        }
        let name_rva = u32::from_le_bytes(desc[12..16].try_into().unwrap());
        if let Some(name_off) = rva_to_offset(name_rva, sections) {
            if let Some(name) = read_ascii_z(content, name_off) {
                imports.push(name);
            }
        }
        offset += 20;
    }
    imports
}

fn parse_pe(content: &mut dyn Content<MyTypes>) -> Option<(u16, u16, u16, Vec<String>)> {
    let e_lfanew = read_u32(content, 0x3C)? as u64;
    let mut sig = [0u8; 4];
    if content.read_into(e_lfanew, 4, &mut sig) != Some(4) || &sig != b"PE\0\0" {
        return None;
    }

    let machine = read_u16(content, e_lfanew + 4)?;
    let nsections = read_u16(content, e_lfanew + 6)?;
    let opt_size = read_u16(content, e_lfanew + 20)?;
    let opt = e_lfanew + 24;
    let magic = read_u16(content, opt)?;
    let (bits, dd_base, nrva_off) = match magic {
        PE32_MAGIC => (32u16, opt + 96, opt + 92),
        PE32PLUS_MAGIC => (64u16, opt + 112, opt + 108),
        _ => return None,
    };
    let n_rva = read_u32(content, nrva_off)?;
    let sections = read_sections(content, opt + opt_size as u64, nsections)?;
    let imports = if n_rva > IMPORT_DIR_INDEX as u32 {
        let import_rva = read_u32(content, dd_base + IMPORT_DIR_INDEX * 8)?;
        if import_rva == 0 {
            Vec::new()
        } else {
            read_imports(content, import_rva, &sections)
        }
    } else {
        Vec::new()
    };
    Some((machine, nsections, bits, imports))
}

#[derive(Dependencies)]
#[Dependencies(name = "PeHeader")]
pub struct PeHeaderAnalyzer;

impl ContentAnalyzer<MyTypes> for PeHeaderAnalyzer {
    fn analyze(&mut self, content: &mut dyn Content<MyTypes>, context: &mut Context<MyTypes>) -> AnalysisOutcome<MyTypes> {
        let Some((machine, nsections, bits, imports)) = parse_pe(content) else {
            return AnalysisOutcome::Continue;
        };
        context.add_finding(&format!("pe:sections={nsections}"), Some("PeHeader"), None);
        context.add_finding(&format!("pe:bits={bits}"), Some("PeHeader"), None);
        context.add_finding(&format!("pe:arch={}", machine_name(machine)), Some("PeHeader"), None);
        context.local().set(var!("imports"), imports.join("\n").as_str());
        AnalysisOutcome::Continue
    }
}

#[derive(Dependencies)]
#[Dependencies(name = "DotNetDetector", requires = "PeHeader")]
pub struct DotNetDetector;

impl ContentAnalyzer<MyTypes> for DotNetDetector {
    fn analyze(&mut self, _: &mut dyn Content<MyTypes>, context: &mut Context<MyTypes>) -> AnalysisOutcome<MyTypes> {
        let Some(imports) = context.local().get::<&str>(var!("imports")) else {
            return AnalysisOutcome::Continue;
        };
        let has_mscoree = imports.split('\n').any(|dll| dll.eq_ignore_ascii_case("mscoree.dll"));
        if has_mscoree {
            AnalysisOutcome::ReprocessAsType(MyTypes::DotNet)
        } else {
            AnalysisOutcome::Continue
        }
    }
}
