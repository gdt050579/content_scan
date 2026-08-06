use content_scan::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq, ContentType)]
#[repr(u16)]
enum MyType {
    TextBuffer,
}

struct VowelAnalyzer;
impl ContentAnalyzer<MyType> for VowelAnalyzer {
    fn analyze(&mut self, content: &mut dyn Content<MyType>, context: &mut Context) -> NextAction {
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
        context.global().set(var!("count_vowels"), count);
        NextAction::Continue
    }
}
struct TextBufferItdentifier;
impl ContentIdentifier<MyType> for TextBufferItdentifier {
    fn identify_method(&self) -> Option<IdentifyMethod> {
        Some(IdentifyMethod::Magic(b"TXBF"))
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
    println!("count_vowels: {}", res.global().get::<u32>(var!("count_vowels")).unwrap());
}
