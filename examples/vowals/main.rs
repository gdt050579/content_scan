use content_scan::*;
use varmap::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum MyType {
    TextBuffer,
}
impl ContentType for MyType {
    fn as_u16(&self) -> u16 {
        match self {
            MyType::TextBuffer => 0,
        }
    }
    fn from_u16(value: u16) -> Option<Self> {
        match value {
            0 => Some(MyType::TextBuffer),
            _ => None,
        }
    }
    const COUNT: u16 = 1;
}

struct VowelAnalyzer;
impl ContentAnalyzer<MyType> for VowelAnalyzer {
    fn analyze(&mut self, content: &mut dyn Content<MyType>, output: &mut VarMap) -> NextAction {
        let sz = content.size();
        let mut count = 0u32;
        for i in 4..sz as u64 {
            if let Some(b) = content.read(i, 1) {
                let b = b[0].to_ascii_lowercase();
                if b == b'a' || b == b'e' || b == b'i' || b == b'o' || b == b'u' {
                    count += 1;
                }
            }
        }
        output.set(var!("count_vowels"), count);
        NextAction::Continue
    }
}
struct TextBufferItdentifier;
impl ContentIdentifier<MyType> for TextBufferItdentifier {
    fn fast_id(&self) -> Option<FastID> {
        Some(FastID::Magic(b"TXBF"))
    }

    fn validate(&self, _: &dyn Content<MyType>) -> bool {
        true
    }
}

fn main() {
    let mut scanner = ScannerBuilder::new()
        .add_analyzer(MyType::TextBuffer, 0, VowelAnalyzer {})
        .add_identifier(MyType::TextBuffer, TextBufferItdentifier {})
        .build();
    let mut b = BufferContent::<MyType>::with_content_type(b"TXBF   Hellow World !", "test.txt", MyType::TextBuffer);
    let res = scanner.scan(&mut b);
    println!("count_vowels: {}", res.get::<u32>(var!("count_vowels")).unwrap());
}
