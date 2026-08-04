use super::{Content, ContentType};
use varmap::VarMap;

pub enum NextAction {
    Continue,
    Skip,
    Exit,
}

pub struct Entry {
    pub path: String,
    pub size: u64,
}

pub trait Analyzer<T: ContentType> {
    fn analyze(&mut self, content: &mut dyn Content<T>, output: &mut VarMap) -> NextAction;
}
pub trait Extractor<T: ContentType> {
    fn init(&mut self, content: &mut dyn Content<T>, map: &VarMap) -> bool;
    fn advance(&mut self, content: &mut dyn Content<T>) -> Option<&Entry>;
    fn extract(&mut self, content: &mut dyn Content<T>) -> Option<Box<dyn Content<T>>>;
}

pub trait ContentAnalyzer<T: ContentType> {
    fn analyze(&mut self, content: &dyn Content<T>, output: &mut VarMap) -> AnalysisResult;
    fn init_entry(&mut self, content: &mut dyn Content<T>, entry: &mut Entry) -> bool;
    fn next_entry(&mut self, content: &mut dyn Content<T>, entry: &mut Entry) -> bool;
    fn extract_entry(
        &mut self,
        content: &mut dyn Content<T>,
        entry: &Entry,
    ) -> Option<Box<dyn Content<T>>>;
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
