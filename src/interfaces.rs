use std::any::Any;

pub enum ObjectType {
}

pub trait Object {
    fn types(&self) -> &[ObjectType];
    fn path(&self) -> &str;
    fn size(&self) -> u64;
    fn read(&mut self, offset: u64, count: u32) -> Option<&[u8]>;
}
pub trait Analyzer {
    fn analyze(&mut self, object: &dyn Object);
}

pub enum EntryCursor {
    Offset(u64),
    Pair(u64, u64),  
    Folder(Box<std::fs::ReadDir>),
    Generic(Box<dyn Any>),
}

pub struct Entry {
    name: String,
    size: Option<u64>, 
    cursor: EntryCursor,
}

pub trait Extractor {
    fn init(&mut self, object: &mut dyn Object) -> Option<Entry>;
    fn next(&mut self, object: &mut dyn Object, entry: &mut Entry) -> bool;
    fn extract(&mut self, object: &mut dyn Object, entry: &Entry) -> Option<Box<dyn Object>>;
}

pub trait Probe {
    fn magic() -> &'static [u8];
    fn update_types(object: &mut dyn Object); // adds ObjectTypes to object
}