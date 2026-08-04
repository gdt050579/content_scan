use std::any::Any;
use super::{Content, ContentType};
use varmap::VarMap;

pub enum AnalysisResult {
    Continue,
    Stop,
    Extract
}

pub trait ContentAnalyzer<T: ContentType> {
    fn analyze(&mut self, content: &dyn Content<T>, output: &mut VarMap) -> AnalysisResult;
    fn init_entry(&mut self, content: &mut dyn Content<T>,entry: &mut Entry) -> bool;
    fn next_entry(&mut self, content: &mut dyn Content<T>, entry: &mut Entry) -> bool;
    fn extract_entry(&mut self, content: &mut dyn Content<T>, entry: &Entry) -> Option<Box<dyn Content<T>>>;
}

pub enum EntryCursor {
    Offset(u64),
    Pair(u64, u64),  
    Folder(Box<std::fs::ReadDir>),
    Generic(Box<dyn Any>),
}

pub struct Entry {
    pub path: String,
    pub size: Option<u64>, 
    pub cursor: EntryCursor,
}

pub enum FastID {
    Magic(&'static [u8]),
    MultipleMagic(&'static [&'static [u8]]),
    Extension(&'static str),
    Extensions(&'static [&'static str]),
    Name(&'static str),
    Names(&'static [&'static str]),
}

pub trait ContentIdentifier<T: ContentType> {
    fn fast_id(&self) -> Option<FastID>;
    fn validate(&self, content: &dyn Content<T>) -> bool;
}