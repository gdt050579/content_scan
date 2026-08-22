use crate::object::ArenaIndex;

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
impl<M: FindingMetadata> InternalFinding<M> {

}