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
struct ExtractData {
    pos: u64,
    start: u64,
    len: u64,
}
#[derive(Default)]
struct NumericExtractor {
    e: ExtractionPool<ExtractData>,
    entry: Entry,
}
impl ContentExtractor<MyTypes> for NumericExtractor {
    fn acquire(&mut self, _: &mut dyn Content<MyTypes>, _: &mut VarMap) -> Option<ExtractionHandle> {
        Some(self.e.acquire_slot(ExtractData { pos: 0, start: u64::MAX, len: 0 }))
    }
    fn advance(&mut self, handle: ExtractionHandle, content: &mut dyn Content<MyTypes>) -> Option<&Entry> {
        let data = self.e.get_mut(handle)?;
        data.start = u64::MAX;
        while data.pos < content.size() {
            if let Some(b) = content.read(data.pos, 1) {
                if b[0].is_ascii_digit() {
                    data.start = data.pos;
                    break;
                }
            }
            data.pos += 1;
        }
        if data.start == u64::MAX {
            return None;
        }
        data.len = 0;
        while data.pos < content.size() {
            if let Some(b) = content.read(data.pos, 1) {
                if b[0].is_ascii_digit() {
                    data.len += 1;
                    data.pos += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        if data.len == 0 {
            return None;
        }
        let len = data.len;
        self.entry.path.set_from_str("number");
        self.entry.size = len;
        self.entry.skip_from_filtering = false;
        Some(&self.entry)
    }
    fn extract(&mut self, handle: ExtractionHandle, content: &mut dyn Content<MyTypes>) -> Option<Box<dyn Content<MyTypes>>> {
        let data = self.e.get(handle)?;
        if let Some(buf) = content.read(data.start, data.len as u32) {
            let extr = BufferContent::<MyTypes>::with_content_type(buf, "number", MyTypes::Number);
            Some(Box::new(extr))
        } else {
            None
        }
    }
    fn release(&mut self, handle: ExtractionHandle) {
        self.e.release_slot(handle);
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
    let mut b = BufferContent::<MyTypes>::new(b"TXT   10+20+30=", "test.txt");
    let res = scanner.scan(&mut b, true);
    println!("sum: {}", res.global().get::<u32>(var!("sum")).unwrap_or(0));
    // navigate
    let root = res.root().unwrap();
    println!("root: {}", res.path(root).unwrap());
    let mut c = res.child(root).unwrap();
    println!("- child: {} => {}", res.path(c).unwrap(), res.local(c).unwrap().get::<u32>(var!("value")).unwrap_or(0));
    while let Some(next) = res.next_sibling(c) {
        c = next;
        println!("- sibling: {} => {}", res.path(c).unwrap(), res.local(c).unwrap().get::<u32>(var!("value")).unwrap_or(0));
    }
}
