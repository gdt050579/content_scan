use std::fmt::Debug;

pub trait ContentType: Copy + Eq + PartialEq + Debug {
    const COUNT: u16;
    fn as_u16(&self) -> u16;
    fn from_u16(value: u16) -> Option<Self>;
}

pub trait Content<T: ContentType> {
    fn content_type(&self) -> Option<T> {
        None
    }
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

pub struct BufferContent<T: ContentType> {
    buffer: Vec<u8>,
    path: String,
    content_type: Option<T>,
}
impl<T: ContentType> BufferContent<T> {
    pub fn new(buffer: &[u8], path: &str) -> Self {
        Self {
            buffer: buffer.to_vec(),
            path: path.to_string(),
            content_type: None,
        }
    }
    pub fn with_content_type(buffer: &[u8], path: &str, content_type: T) -> Self {
        Self {
            buffer: buffer.to_vec(),
            path: path.to_string(),
            content_type: Some(content_type),
        }
    }
    pub fn from_parts(buffer: Vec<u8>, path: String, content_type: Option<T>) -> Self {
        Self {
            buffer,
            path,
            content_type,
        }
    }
}
impl<T: ContentType> Content<T> for BufferContent<T> {
    #[inline(always)]
    fn content_type(&self) -> Option<T> {
        self.content_type
    }
    #[inline(always)]
    fn path(&self) -> &str {
        &self.path
    }
    #[inline(always)]
    fn size(&self) -> u64 {
        self.buffer.len() as u64
    }
    fn read(&mut self, offset: u64, count: u32) -> Option<&[u8]> {
        if offset > self.buffer.len() as u64 {
            return None;
        }
        if offset == self.buffer.len() as u64 {
            return Some(&[]);
        }
        let len = (self.buffer.len() as u64 - offset).min(count as u64) as usize;
        Some(&self.buffer.as_slice()[offset as usize..offset as usize + len])
    }
}