use crate::object::ArenaIndex;

pub(crate) struct BufferArena {
    buf: Vec<u8>,
}
impl BufferArena {
    pub(crate) fn new() -> Self {
        Self { buf: Vec::new() }
    }
    pub(crate) fn clear(&mut self) {
        self.buf.clear();
    }
    pub(crate) fn alloc(&mut self, data: &[u8]) -> ArenaIndex {
        let index = self.buf.len();
        self.buf.extend_from_slice(data);
        ArenaIndex {
            pos: index as u32,
            size: data.len() as u32,
        }
    }
    pub(crate) fn get(&self, index: ArenaIndex) -> Option<&[u8]> {
        let end = index.pos.saturating_add(index.size) as usize;
        if end > self.buf.len() {
            None
        } else {
            Some(&self.buf[index.pos as usize..end])
        }
    }
}
