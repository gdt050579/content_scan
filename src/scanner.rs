pub struct Scannr {
    filter: Option<Fn(&str,u32) -> bool>,
    analyzers: Vec<Box<dyn Analyzer>>,
    extractors: Vec<Box<dyn Extractor>>,
}
impl Scanner {
    pub fn scan(&mut self, object: &dyn Object) {
        self.inner_scan(object, 0);
    }
    fn inner_scan(&mut self, object: &dyn Object, depth: u32) {
        if let Some(filter) = &self.filter {
            if !filter(object.path(), object.size()) {
                return;
            }
        }
        // citesc 16 octeti de la inceput
        let start_offset = object.read(0, 16).unwrap();
        // validez magic
        for analyzer in &mut self.analyzers {
            analyzer.analyze(object);
        }
        for extractor in &mut self.extractors {
            // extract the object and call inner_scan on the extracted object
        }
    }
}
pub struct ScannerBuilder {
    filter: Option<Fn(&str,u32) -> bool>,
    analyzers: Vec<(ObjectType, Box<dyn Analyzer>)>,
    extractors: Vec<(ObjectType, Box<dyn Extractor>)>,
}
impl ScannerBuilder {
    pub fn new() -> Self {
        Self {
            filter: None,
            analyzers: Vec::new(),
            extractors: Vec::new(),
        }
    }
    pub fn filter(mut self, filter: Fn(&str,u32) -> bool) -> Self {
        self.filter = Some(filter);
        self
    }
    pub fn register_analyzer(mut self, object_type: ObjectType, analyzer: Box<dyn Analyzer>) -> Self {
        self.analyzers.push((object_type, analyzer));
        self
    }
    pub fn register_extractor(mut self, object_type: ObjectType, extractor: Box<dyn Extractor>) -> Self {
        self.extractors.push((object_type, extractor));
        self
    }
    pub fn build(self) -> Scanner {
        Scanner {
            analyzers: self.analyzers,
            extractors: self.extractors,
        }
    }
}