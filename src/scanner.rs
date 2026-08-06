use std::collections::HashMap;

use varmap::VarMap;

use super::{Content, ContentAnalyzer, ContentExtractor,ContentIdentifier, ContentType, Entry, Filter, NextAction, plugin_list::PluginsList};
use crate::Matcher;
pub struct Scanner<T: ContentType> {
    magics: Matcher<T>,
    extensions: Matcher<T>,
    names: Matcher<T>,
    filter: Filter,
    identifiers: Vec<Box<dyn ContentIdentifier<T>>>,
    analyzers: PluginsList<Box<dyn ContentAnalyzer<T>>>,
    extractors: PluginsList<Box<dyn ContentExtractor<T>>>,
    varm: VarMap,
}
impl<T: ContentType> Scanner<T> {
    pub fn scan(&mut self, content: &mut dyn Content<T>) {
        if !self.filter.should_process(content.path(), 0, content.size()) {
            return;
        }
        self.inner_scan(content, 0);
    }
    fn inner_scan(&mut self, content: &mut dyn Content<T>, depth: u32) -> NextAction {
        let ty = self.retrieve_content_type(content);
        let range = if let Some(ty) = ty {
            self.analyzers.range(ty)
        } else {
            None
        };
        if let Some((start, end)) = range {
            match self.scan_range(content, start, end) {
                NextAction::Continue => {},
                NextAction::Skip => return NextAction::Continue, // skip current content
                NextAction::Exit => return NextAction::Exit,
            }
        }
        // generic analyzers
        let range = self.analyzers.generic_range();
        if let Some((start, end)) = range {
            match self.scan_range(content, start, end) {
                NextAction::Continue => {},
                NextAction::Skip => return NextAction::Continue, // skip current content
                NextAction::Exit => return NextAction::Exit,
            }
        }
        // extractors (specfic)
        if let Some(ty) = ty {
            if let Some((start,end)) = self.extractors.range(ty) {
                match self.extract_range(content, start, end, depth) {
                    NextAction::Continue => {},
                    NextAction::Skip => return NextAction::Continue, // skip current content
                    NextAction::Exit => return NextAction::Exit,
                }
            }
        }
        // extractors (generc)
        let range = self.extractors.generic_range();
        if let Some((start, end)) = range {
            match self.extract_range(content, start, end, depth) {
                NextAction::Continue => {},
                NextAction::Skip => return NextAction::Continue, // skip current content
                NextAction::Exit => return NextAction::Exit,
            }
        }
        NextAction::Continue
    }
    fn scan_range(&mut self, content: &mut dyn Content<T>, start: usize, end: usize) -> NextAction {
        if (end <= start) || (end > self.analyzers.len()) {
            return NextAction::Continue;
        }
        for i in start..end {
            let result = unsafe { self.analyzers.get(i).analyze(content, &mut self.varm) };
            match result {
                NextAction::Continue => continue,
                NextAction::Exit => return NextAction::Exit,
                NextAction::Skip => return NextAction::Skip,
            }
        }
        NextAction::Continue
    }
    fn extract_range(&mut self, content: &mut dyn Content<T>, start: usize, end: usize, depth: u32) -> NextAction {
        if (end <= start) || (end > self.analyzers.len()) {
            return NextAction::Continue;
        }
        for i in start..end {
            let result = self.extract_content(content, i, depth);
            match result {
                NextAction::Continue => continue,
                NextAction::Exit => return NextAction::Exit,
                NextAction::Skip => return NextAction::Skip,
            }
        }
        NextAction::Continue        
    }
    fn extract_content(&mut self, content: &mut dyn Content<T>, index: usize, depth: u32) -> NextAction {
        let len = self.extractors.len();
        if index >= len {
            return NextAction::Continue;
        }
        let mut extractor = unsafe { self.extractors.get(index) };
        if !extractor.init(content, &mut self.varm) {
            return NextAction::Continue;
        }
        while let Some(entry) = unsafe { self.extractors.get(index).advance(content) } {
            if !self.filter.should_process(&entry.path, depth + 1, entry.size) {
                continue;
            }
            extractor = unsafe { self.extractors.get(index) };
            if let Some(mut extracted_content) = extractor.extract(content) {
                let result = self.inner_scan(&mut *extracted_content, depth + 1);
                match result {
                    NextAction::Continue => continue,
                    NextAction::Exit => return NextAction::Exit,
                    NextAction::Skip => return NextAction::Continue,
                }
            }
        }
        NextAction::Continue
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
    analyzers: Vec<(u32, Box<dyn ContentAnalyzer<T>>)>,
    extractors: Vec<(u32, Box<dyn ContentExtractor<T>>)>,
    identifiers: Vec<(T, Box<dyn ContentIdentifier<T>>)>,
}
impl<T: ContentType> ScannerBuilder<T> {
    pub fn new() -> Self {
        Self {
            filter: Filter::new(),
            analyzers: Vec::with_capacity(16),
            extractors: Vec::with_capacity(4),
            identifiers: Vec::with_capacity(4),
        }
    }
    pub fn filter(mut self, filter: Filter) -> Self {
        self.filter = filter;
        self
    }
    pub fn add_analyzer<A>(mut self, content_type: T, priority: u8, analyzer: A) -> Self
    where
        A: ContentAnalyzer<T> + 'static,
    {
        let hash = (content_type.as_u16() as u32) << 16 | priority as u32;
        self.analyzers.push((hash, Box::new(analyzer)));
        self
    }
    pub fn add_generic_analyzer<A>(mut self, priority: u8, analyzer: A) -> Self
    where
        A: ContentAnalyzer<T> + 'static,
    {
        let hash = 0xFFFF0000 | priority as u32;
        self.analyzers.push((hash, Box::new(analyzer)));
        self
    }
    pub fn add_extractor<E>(mut self, content_type: T, priority: u8, extractor: E) -> Self
    where
        E: ContentExtractor<T> + 'static,
    {
        let hash = (content_type.as_u16() as u32) << 16 | priority as u32;
        self.extractors.push((hash, Box::new(extractor)));
        self
    }
    pub fn add_generic_extractor<E>(mut self, priority: u8, extractor: E) -> Self
    where
        E: ContentExtractor<T> + 'static,
    {
        let hash = 0xFFFF0000 | priority as u32;
        self.extractors.push((hash, Box::new(extractor)));
        self
    }
    pub fn add_identifier<I>(mut self, content_type: T, identifier: I) -> Self
    where
        I: ContentIdentifier<T> + 'static,
    {
        self.identifiers.push((content_type, Box::new(identifier)));
        self
    }
    fn check_consistency(&self) {
        let mut m = HashMap::new();
        for (h, _) in &self.analyzers {
            m.insert((h >> 16) as u16, 1);
        }
        for (h, _) in &self.extractors {
            m.insert((h >> 16) as u16, 1);
        }
        for (content_type, _) in &self.identifiers {
            let id = content_type.as_u16() as u16;
            if let Some(mask) = m.get_mut(&id) {
                if (*mask) == 3 {
                    panic!("There can only be one identifier for type ! Type {:?} has multiple identifiers !", content_type);
                }
                *mask = 3;
            } else {
                m.insert(id, 2);
            }
        }
        // ar trebui toate sa fie cu 3
        for (id, mask) in m {
            if mask == 1 {
                panic!("For type {:?}, there is an analyzer/extractor but no identifier !", T::from_u16(id as u16).unwrap());
            }
            if mask == 2 {
                panic!("For type {:?}, there is an identifier but no analyzer/extractor !", T::from_u16(id as u16).unwrap());
            }
        }
    }
    pub fn build(self) -> Scanner<T> {
        self.check_consistency();
        let analyzers = PluginsList::new(self.analyzers, T::COUNT);
        let extractors = PluginsList::new(self.extractors, T::COUNT);
        // build-ul de identificatori
        // verificat sa nu am extractor/analyzator fara identficatori sau invers
        todo!()
    }
}
