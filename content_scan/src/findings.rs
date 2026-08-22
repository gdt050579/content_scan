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
