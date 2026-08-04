use super::{ContentAnalyzer, ContentType};

pub(super) struct AnalyzerList<T: ContentType> {
    analyzers: Vec<(T, Box<dyn ContentAnalyzer<T>>)>,
}
impl<T: ContentType> AnalyzerList<T> {
    pub fn new() -> Self {
        Self {
            analyzers: Vec::new(),
        }
    }
    pub(super) fn available_analyzers_range(&self, content_type: T) -> Option<(usize,usize)> {
        todo!();
    }
    pub(super) fn generic_analyzers_range(&self) -> Option<(usize,usize)> {
        todo!();
    }
    pub(super) fn add(&mut self, content_type: T, priority: u8, analyzer: Box<dyn ContentAnalyzer<T>>) {
        todo!();
    }
    pub(super) fn add_generic_analyzer(&mut self, priority: u8, analyzer: Box<dyn ContentAnalyzer<T>>) {
        todo!();
    }
    pub(super) fn get(&mut self, index: usize) -> Option<&mut Box<dyn ContentAnalyzer<T>>> {
        todo!();
    }
}
