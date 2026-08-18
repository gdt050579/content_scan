use content_scan::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ContentType)]
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

    fn validate(&self, _: &mut dyn Content<MyTypes>) -> bool {
        true
    }
}

struct NumericExtractor;

struct NumericSession {
    content: OwnedContentPtr<MyTypes>,
    pos: u64,
    start: u64,
    len: u64,
    entry: Entry,
}

impl ContentExtractor<MyTypes> for NumericExtractor {
    fn create_session(&mut self, content: OwnedContentPtr<MyTypes>, _: &ExtractionContext) -> Option<Box<dyn ExtractionSession<MyTypes>>> {
        Some(Box::new(NumericSession {
            content,
            pos: 0,
            start: u64::MAX,
            len: 0,
            entry: Entry::default(),
        }))
    }
}

impl ExtractionSession<MyTypes> for NumericSession {
    fn advance(&mut self) -> Option<&Entry> {
        self.start = u64::MAX;
        while self.pos < self.content.size() {
            if let Some(b) = self.content.read(self.pos, 1) {
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
        while self.pos < self.content.size() {
            if let Some(b) = self.content.read(self.pos, 1) {
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
        self.entry.path.set_from_str("number");
        self.entry.size = self.len;
        self.entry.skip_from_filtering = false;
        Some(&self.entry)
    }
    fn extract(&mut self) -> Option<Box<dyn Content<MyTypes>>> {
        if let Some(buf) = self.content.read(self.start, self.len as u32) {
            let extr = BufferContent::<MyTypes>::with_content_type(buf, "number", MyTypes::Number);
            Some(Box::new(extr))
        } else {
            None
        }
    }
}

struct NumericAnalyzer;
impl ContentAnalyzer<MyTypes> for NumericAnalyzer {
    fn analyze(&mut self, content: &mut dyn Content<MyTypes>, context: &mut Context<MyTypes>) -> NextAction {
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
        .add_extractor(MyTypes::Text, NumericExtractor)
        .add_identifier(MyTypes::Text, TextIdentifier {})
        .build();
    let mut b = BufferContent::<MyTypes>::new(b"TXT   10+20+30=", "test.txt");
    let res = scanner.scan(&mut b, true);
    println!("sum: {}", res.global().get::<u32>(var!("sum")).unwrap_or(0));
    // navigate
    let root = res.root().unwrap();
    println!("root: {}", res.path(root).unwrap());
    let mut c = res.child(root).unwrap();
    println!(
        "- child: {} => {}",
        res.path(c).unwrap(),
        res.local(c).unwrap().get::<u32>(var!("value")).unwrap_or(0)
    );
    while let Some(next) = res.next_sibling(c) {
        c = next;
        println!(
            "- sibling: {} => {}",
            res.path(c).unwrap(),
            res.local(c).unwrap().get::<u32>(var!("value")).unwrap_or(0)
        );
    }
}
