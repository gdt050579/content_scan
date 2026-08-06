use std::collections::HashSet;
use crate::utils;
use crate::{Context, ScanResult};
use super::{Content, ContentAnalyzer, ContentExtractor, ContentIdentifier, ContentType, Filter, NextAction, plugin_list::PluginsList};
use crate::IdentifierSet;
pub struct Scanner<T: ContentType> {
    filter: Option<Filter>,
    identifiers: IdentifierSet<T>,
    analyzers: PluginsList<Box<dyn ContentAnalyzer<T>>>,
    extractors: PluginsList<Box<dyn ContentExtractor<T>>>,
    context: Context,
    max_depth: u32,
}
impl<T: ContentType> Scanner<T> {
    pub fn scan<'a>(&'a mut self, content: &mut dyn Content<T>) -> ScanResult<'a> {
        self.context.clear();
        if let Some(filter) = &self.filter {
            if !filter.should_process(content.path(), content.size()) {
                return ScanResult::new(&self.context);
            }
        }
        self.inner_scan(content, 1);
        ScanResult::new(&self.context)
    }
    fn inner_scan(&mut self, content: &mut dyn Content<T>, depth: u32) -> NextAction {
        self.context.clear_extract();
        let ty = self.retrieve_content_type(content);
        let range = if let Some(ty) = ty { self.analyzers.range(ty) } else { None };
        if let Some((start, end)) = range {
            match self.scan_range(content, start, end) {
                NextAction::Continue => {}
                NextAction::Skip => return NextAction::Continue, // skip current content
                NextAction::Exit => return NextAction::Exit,
            }
        }
        // generic analyzers
        let range = self.analyzers.generic_range();
        if let Some((start, end)) = range {
            match self.scan_range(content, start, end) {
                NextAction::Continue => {}
                NextAction::Skip => return NextAction::Continue, // skip current content
                NextAction::Exit => return NextAction::Exit,
            }
        }
        // extractors (specfic)
        if let Some(ty) = ty {
            if let Some((start, end)) = self.extractors.range(ty) {
                match self.extract_range(content, start, end, depth) {
                    NextAction::Continue => {}
                    NextAction::Skip => return NextAction::Continue, // skip current content
                    NextAction::Exit => return NextAction::Exit,
                }
            }
        }
        // extractors (generc)
        let range = self.extractors.generic_range();
        if let Some((start, end)) = range {
            match self.extract_range(content, start, end, depth) {
                NextAction::Continue => {}
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
            let result = unsafe { self.analyzers.get(i).analyze(content, &mut self.context) };
            match result {
                NextAction::Continue => continue,
                NextAction::Exit => return NextAction::Exit,
                NextAction::Skip => return NextAction::Skip,
            }
        }
        NextAction::Continue
    }
    fn extract_range(&mut self, content: &mut dyn Content<T>, start: usize, end: usize, depth: u32) -> NextAction {
        if (end <= start) || (end > self.extractors.len()) {
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
        if depth > self.max_depth {
            return NextAction::Continue;
        }
        let len = self.extractors.len();
        if index >= len {
            return NextAction::Continue;
        }
        let mut extractor = unsafe { self.extractors.get(index) };
        if !extractor.init(content, &mut self.context.extract()) {
            return NextAction::Continue;
        }
        while let Some(entry) = unsafe { self.extractors.get(index).advance(content) } {
            if let Some(filter) = &self.filter {
                if !filter.should_process(&entry.path, entry.size) {
                    continue;
                }
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
        if let Some(ty) = content.content_type() {
            return Some(ty);
        }
        let p = content.path().as_bytes();
        // type from file name
        let file_name = utils::get_file_name(p);
        let type_from_file_name = self.identifiers.type_from_file_name(file_name);
        let extension = utils::get_extension(file_name);
        let type_from_extension = self.identifiers.type_from_extension(extension);
        // type from magic
        let type_from_magic = {
            if let Some(buf) = content.read(0, 16) {
                self.identifiers.type_from_magic(buf)
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
    #[inline(always)]
    fn validate_content_type(&self, content: &mut dyn Content<T>, content_type: T) -> bool {
        self.identifiers
            .get(content_type)
            .map(|identifier| identifier.validate(content))
            .unwrap_or(false)
    }
}
pub struct ScannerBuilder<T: ContentType> {
    filter: Option<Filter>,
    analyzers: Vec<(u32, Box<dyn ContentAnalyzer<T>>)>,
    extractors: Vec<(u32, Box<dyn ContentExtractor<T>>)>,
    identifiers: Vec<(T, Box<dyn ContentIdentifier<T>>)>,
    max_depth: u32,
}
impl<T: ContentType> ScannerBuilder<T> {
    pub fn new() -> Self {
        Self {
            filter: None,
            analyzers: Vec::with_capacity(16),
            extractors: Vec::with_capacity(4),
            identifiers: Vec::with_capacity(4),
            max_depth: 8,
        }
    }
    pub fn filter(mut self, filter: Filter) -> Self {
        self.filter = Some(filter);
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
    pub fn max_depth(mut self, max_depth: u32) -> Self {
        self.max_depth = max_depth.clamp(1, u32::MAX-2);
        self
    }
    fn check_unique_identifiers(&self) {
        let mut m = HashSet::new();
        for (content_type, _) in &self.identifiers {
            if m.contains(&content_type.as_u16()) {
                panic!(
                    "There can only be one identifier for type ! Type {:?} has multiple identifiers !",
                    content_type
                );
            }
            m.insert(content_type.as_u16());
        }
    }
    pub fn build(self) -> Scanner<T> {
        self.check_unique_identifiers();
        let analyzers = PluginsList::new(self.analyzers, T::COUNT);
        let extractors = PluginsList::new(self.extractors, T::COUNT);
        let identifiers = IdentifierSet::new(self.identifiers);
        Scanner {
            filter: self.filter,
            identifiers,
            analyzers,
            extractors,
            context: Context::new(),
            max_depth: self.max_depth,
        }
    }
}
