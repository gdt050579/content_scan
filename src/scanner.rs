use varmap::VarMap;

use super::{
    AnalysisResult, Content, ContentAnalyzer, ContentIdentifier, ContentType, Entry, EntryCursor,
    Filter, analyzer_list::AnalyzerList,
};
use crate::Matcher;
pub struct Scanner<T: ContentType> {
    magics: Matcher<T>,
    extensions: Matcher<T>,
    names: Matcher<T>,
    filter: Filter,
    identifiers: Vec<Box<dyn ContentIdentifier<T>>>,
    analyzers: AnalyzerList<T>,
    varm: VarMap,
}
impl<T: ContentType> Scanner<T> {
    pub fn scan(&mut self, content: &mut dyn Content<T>) {
        if !self
            .filter
            .should_process(content.path(), 0, content.size())
        {
            return;
        }
        self.inner_scan(content, 0);
    }
    fn inner_scan(&mut self, content: &mut dyn Content<T>, depth: u32) {
        let ty = self.retrieve_content_type(content);
        let range = if let Some(ty) = ty {
            self.analyzers.available_analyzers_range(ty)
        } else {
            None
        };
        if let Some((start, end)) = range {
            self.scan_range(content, start, end, depth);
        }
        // generic analyzers
        let range = self.analyzers.generic_analyzers_range();
        if let Some((start, end)) = range {
            self.scan_range(content, start, end, depth);
        }
    }
    fn scan_range(&mut self, content: &mut dyn Content<T>, start: usize, end: usize, depth: u32) {
        if (end <= start) || (end > self.analyzers.len()) {
            return;
        }
        for i in start..end {
            let result = unsafe { self.analyzers.get(i).analyze(content, &mut self.varm) };
            match result {
                AnalysisResult::Continue => continue,
                AnalysisResult::Stop => break,
                AnalysisResult::Extract => {
                    return;
                }
            }
        }
    }
    fn extract_content(&mut self, content: &mut dyn Content<T>, index: usize, depth: u32) {
        let len = self.analyzers.len();
        if index >= len {
            return;
        }
        let mut entry = Entry {
            path: String::new(),
            size: Some(0),
            cursor: EntryCursor::Offset(0),
        };
        let inited = unsafe { self.analyzers.get(index).init_entry(content, &mut entry) };
        if !inited {
            return;
        }
        loop {
            let should_process =
                self.filter
                    .should_process(&entry.path, depth + 1, entry.size.unwrap_or(0));
            if should_process {
                if let Some(mut extracted_content) =
                    unsafe { self.analyzers.get(index).extract_entry(content, &entry) }
                {
                    self.inner_scan(&mut *extracted_content, depth + 1);
                }
            }
            if !unsafe { self.analyzers.get(index).next_entry(content, &mut entry) } {
                break;
            }
        }
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
pub struct ScannerBuilder<T: ContentType> {
    filter: Filter,
    analyzers: AnalyzerList<T>,
    identifiers: Vec<Box<dyn ContentIdentifier<T>>>,
    identifiers_type: Vec<T>,
}
impl<T: ContentType> ScannerBuilder<T> {
    pub fn new() -> Self {
        Self {
            filter: Filter::new(),
            analyzers: AnalyzerList::new(),
            identifiers: Vec::new(),
            identifiers_type: Vec::new(),
        }
    }
    pub fn filter(mut self, filter: Filter) -> Self {
        self.filter = filter;
        self
    }
    pub fn add_analyzer(
        mut self,
        content_type: T,
        priority: u8,
        analyzer: Box<dyn ContentAnalyzer<T>>,
    ) -> Self {
        self.analyzers.add(content_type, priority, analyzer);
        self
    }
    pub fn add_generic_analyzer(
        mut self,
        priority: u8,
        analyzer: Box<dyn ContentAnalyzer<T>>,
    ) -> Self {
        self.analyzers.add_generic_analyzer(priority, analyzer);
        self
    }
    pub fn add_identifier(
        mut self,
        content_type: T,
        identifier: Box<dyn ContentIdentifier<T>>,
    ) -> Self {
        self.identifiers.push(identifier);
        self.identifiers_type.push(content_type);
        self
    }
    pub fn build(self) -> Scanner<T> {
        todo!()
    }
}
