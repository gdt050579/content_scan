use std::fmt::Debug;

pub trait ContentType: Copy + Eq + PartialEq + Debug{
    const COUNT: u16;
    fn as_u16(&self) -> u16;
    fn from_u16(value: u16) -> Option<Self>;
}

pub trait Content<T: ContentType> {
    fn content_type(&self) -> Option<T> { None }
    fn path(&self) -> &str;
    fn size(&self) -> u64;
    fn read(&mut self, offset: u64, count: u32) -> Option<&[u8]>;
}

impl ContentType for bool {
    const COUNT: u16 = 2;
    fn as_u16(&self) -> u16 {
        *self as u16
    }
    fn from_u16(value: u16) -> Option<Self> {
        if value == 0 {
            Some(false)
        } else if value == 1 {
            Some(true)
        } else {
            None
        }
    }
}