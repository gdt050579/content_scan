use super::{Content, ContentType, Context};
use varmap::VarMap;

pub enum NextAction {
    Continue,
    Skip,
    Exit,
}

#[derive(Default)]
pub struct Entry {
    pub path: String,
    pub size: u64,
}

pub trait ContentAnalyzer<T: ContentType> {
    fn analyze(&mut self, content: &mut dyn Content<T>, context: &mut Context) -> NextAction;
}
pub trait ContentExtractor<T: ContentType> {
    fn init(&mut self, content: &mut dyn Content<T>, extract_context: &mut VarMap) -> bool;
    fn advance(&mut self, content: &mut dyn Content<T>) -> Option<&Entry>;
    fn extract(&mut self, content: &mut dyn Content<T>) -> Option<Box<dyn Content<T>>>;
}


pub enum IdentifyMethod {
    Magic(&'static [u8]),
    MultipleMagic(&'static [&'static [u8]]),
    Extension(&'static str),
    Extensions(&'static [&'static str]),
    Name(&'static str),
    Names(&'static [&'static str]),
}

pub trait ContentIdentifier<T: ContentType> {
    fn identify_method(&self) -> Option<IdentifyMethod>;
    fn validate(&self, content: &dyn Content<T>) -> bool;
}
