use content_scan::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq, ContentType)]
#[repr(u16)]
enum MyTypes {
    Number,
    Text,
}

struct TextIdentifier;
impl ContentIdentifier<MyTypes> for TextIdentifier {
    fn identify_method(&self) -> Option<IdentifyMethod> {
        Some(IdentifyMethod::Magic(b"TXT"))
    }

    fn validate(&self, _: &dyn Content<MyTypes>) -> bool {
        true
    }
}

#[derive(Default)]
struct NumericExtractor {
    pos: u64,
    start: u64,
    len: u64,
    e: Entry,
}
impl ContentExtractor<MyTypes> for NumericExtractor {
    fn init(&mut self, _: &mut dyn Content<MyTypes>, _: &mut VarMap) -> bool {
        self.pos = 0;
        true
    }
    fn advance(&mut self, content: &mut dyn Content<MyTypes>) -> Option<&Entry> {
        self.start = u64::MAX;
        while self.pos < content.size() {
            if let Some(b) = content.read(self.pos, 1) {
                if b[0].is_ascii_digit() {
                    self.start = self.pos;
                    break;
                }
            }
            self.pos += 1;
        }
        if self.start == u64::MAX {
            return None;
        }
        self.len = 0;
        while self.pos < content.size() {
            if let Some(b) = content.read(self.pos, 1) {
                if b[0].is_ascii_digit() {
                    self.len += 1;
                    self.pos += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        if self.len == 0 {
            return None;
        }
        self.e.path = "number".to_string();
        self.e.size = self.len;
        Some(&self.e)
    }
    fn extract(&mut self, content: &mut dyn Content<MyTypes>) -> Option<Box<dyn Content<MyTypes>>> {
        if let Some(buf) = content.read(self.start, self.len as u32) {
            let extr = BufferContent::<MyTypes>::with_content_type(buf, "number", MyTypes::Number);
            Some(Box::new(extr))
        } else {
            None
        }
    }
}

struct NumericAnalyzer;
impl ContentAnalyzer<MyTypes> for NumericAnalyzer {
    fn analyze(&mut self, content: &mut dyn Content<MyTypes>, context: &mut Context) -> NextAction {
        let value = u32::from_str_radix(std::str::from_utf8(content.read(0, content.size() as u32).unwrap()).unwrap(), 10).unwrap();
        if !context.global().update(var!("sum"), |v: &mut u32| *v += value) {
            context.global().set(var!("sum"), value);
        }
        context.local().set(var!("value"), value);
        NextAction::Continue
    }
}

fn main() {
    let mut scanner = ScannerBuilder::new()
        .add_analyzer(MyTypes::Number, 0, NumericAnalyzer {})
        .add_extractor(MyTypes::Text, 0, NumericExtractor::default())
        .add_identifier(MyTypes::Text, TextIdentifier {})
        .build();
    let mut b = BufferContent::<MyTypes>::new(b"TXT   1+2+3=", "test.txt");
    let res = scanner.scan(&mut b);
    println!("sum: {}", res.global().get::<u32>(var!("sum")).unwrap_or(0));
    // navigate
    let root = res.root().unwrap();
    println!("root: {}", res.path(root).unwrap());
    let mut c = res.child(root).unwrap();
    println!("- child: {} => {}", res.path(c).unwrap(), res.local(c).unwrap().get::<u32>(var!("value")).unwrap_or(0));
    while let Some(next) = res.next_sibling(c) {
        println!("- sibling: {} => {}", res.path(c).unwrap(), res.local(c).unwrap().get::<u32>(var!("value")).unwrap_or(0));
        c = next;
    }
}
