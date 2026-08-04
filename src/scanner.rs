use super::{analyzer_list::AnalyzerList, Content, ContentIdentifier, ContentType};
use crate::Matcher;
pub struct Scanner<T: ContentType> {
    magics: Matcher<T>,
    extensions: Matcher<T>,
    names: Matcher<T>,
    filter: Option<fn(&str, u32) -> bool>,
    identifiers: Vec<Box<dyn ContentIdentifier<T>>>,
    analyzers: AnalyzerList<T>,
}
impl<T: ContentType> Scanner<T> {
    pub fn scan(&mut self, content: &mut dyn Content<T>) {
        self.inner_scan(content, 0);
    }
    fn inner_scan(&mut self, content: &mut dyn Content<T>, depth: u32) {
        if let Some(filter) = self.filter {
            if !filter(content.path(), depth) {
                return;
            }
        }
        let ty = self.retrieve_content_type(content);
        let range = if let Some(ty) = ty {
            self.analyzers.available_analyzers_range(ty)
        } else {
            None
        };
        if let Some((start, end)) = range {
            self.scan_range(content, (start, end));
        }
        // generic analyzers
        let range = self.analyzers.generic_analyzers_range();
        if let Some((start, end)) = range {
            self.scan_range(content, (start, end));
        }
    }
    fn scan_range(&mut self, content: &mut dyn Content<T>, range: (usize,usize)) {
        todo!();
    }

    fn retrieve_content_type(&self, content: &mut dyn Content<T>) -> Option<T> {
        let p = content.path().as_bytes();
        // type from file name
        let file_name = if let Some(ofs) = p.iter().rposition(|&b| b == b'/' || b == b'\\') {
            &p[ofs + 1..]
        } else {
            p
        };
        // type from extension
        let type_from_file_name = if file_name.is_empty() {
            None
        } else {
            self.names.matches_exactly(file_name)
        };
        let extension = if let Some(ofs) = file_name.iter().rposition(|&b| b == b'.') {
            &file_name[ofs + 1..]
        } else {
            &[]
        };
        let type_from_extension = if extension.is_empty() {
            None
        } else {
            self.extensions.matches_exactly(extension)
        };
        // type from magic
        let type_from_magic = {
            if let Some(buf) = content.read(0, 16) {
                self.magics.starts_with(buf)
            } else {
                None
            }
        };
        if let Some(ty) = type_from_magic {
            if self.validate_content_type(content, ty) {
                return Some(ty);
            }
        }
        if let Some(ty) = type_from_file_name {
            if self.validate_content_type(content, ty) {
                return Some(ty);
            }
        }
        if let Some(ty) = type_from_extension {
            if self.validate_content_type(content, ty) {
                return Some(ty);
            }
        }
        None
    }
    fn validate_content_type(&self, content: &mut dyn Content<T>, content_type: T) -> bool {
        self.identifiers
            .get(content_type.as_u16() as usize)
            .map(|identifier| identifier.validate(content))
            .unwrap_or(false)
    }
}
pub struct ScannerBuilder {
    filter: Option<Fn(&str, u32) -> bool>,
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
    pub fn filter(mut self, filter: Fn(&str, u32) -> bool) -> Self {
        self.filter = Some(filter);
        self
    }
    pub fn register_analyzer(
        mut self,
        object_type: ObjectType,
        analyzer: Box<dyn Analyzer>,
    ) -> Self {
        self.analyzers.push((object_type, analyzer));
        self
    }
    pub fn register_extractor(
        mut self,
        object_type: ObjectType,
        extractor: Box<dyn Extractor>,
    ) -> Self {
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
