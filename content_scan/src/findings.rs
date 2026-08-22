use crate::{object::ArenaIndex, ContentType, Context};

pub trait FindingMetadata: Copy {}

#[derive(Copy, Clone, Debug)]
pub struct NoMetadata;
impl FindingMetadata for NoMetadata {}

pub(crate) struct InternalFinding<M: FindingMetadata> {
    pub(crate) objindex: u32,
    pub(crate) finding: ArenaIndex,
    pub(crate) source: ArenaIndex,
    pub(crate) metadata: Option<M>,
}
pub struct Finding<'a, T: ContentType, M: FindingMetadata> {
    inner: &'a InternalFinding<M>,
    ctx: &'a Context<T, M>,
}

impl<'a, T: ContentType, M: FindingMetadata> Finding<'a, T, M> {
    pub fn source(&self) -> Option<&'a str> {
        if self.inner.source.is_valid() {
            self.ctx
                .path_arena
                .get(self.inner.source)
                .map(|s| unsafe { std::str::from_utf8_unchecked(s) })
        } else {
            None
        }
    }
    pub fn finding(&self) -> &'a str {
        self.ctx
            .path_arena
            .get(self.inner.finding)
            .map(|s| unsafe { std::str::from_utf8_unchecked(s) })
            .unwrap_or_default()
    }
    pub fn metadata(&self) -> Option<&'a M> {
        self.inner.metadata.as_ref()
    }
    pub fn content_type(&self) -> Option<T> {
        self.ctx.objects.get(self.inner.objindex as usize).and_then(|f| T::from_u16(f.type_id))
    }
    pub fn path(&self) -> Option<&'a str> {
        if let Some(obj) = self.ctx.objects.get(self.inner.objindex as usize) {
            self.ctx.path_arena.get(obj.path).map(|s| unsafe { std::str::from_utf8_unchecked(s) })
        } else {
            None
        }
    }
}

pub struct FindigsIterator<'a, T: ContentType, M: FindingMetadata> {
    id: u32,
    len: u32,
    ctx: &'a Context<T, M>,
}

impl<'a, T: ContentType, M: FindingMetadata> FindigsIterator<'a, T, M> {
    pub(crate) fn new(ctx: &'a Context<T, M>) -> Self {
        let len = ctx.findings.len() as u32;
        Self { id: 0, len, ctx }
    }
}

impl<'a, T: ContentType, M: FindingMetadata> Iterator for FindigsIterator<'a, T, M> {
    type Item = Finding<'a, T, M>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.id >= self.len {
            return None;
        }
        let innner_data = &self.ctx.findings[self.id as usize];
        self.id += 1;
        Some(Finding {
            inner: innner_data,
            ctx: self.ctx,
        })
    }
}
