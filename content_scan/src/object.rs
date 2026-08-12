#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) struct ArenaIndex {
    pub(crate) pos: u32,
    pub(crate) size: u32,
}
// impl ArenaIndex {
//     const INVALID: ArenaIndex = ArenaIndex { pos: u32::MAX, size: u32::MAX };
// }

#[derive(Copy, Clone, Debug)]
pub(crate) struct Object {
    pub(crate) path: ArenaIndex,
    pub(crate) parent_index: u32,
    pub(crate) next_siblig_index: u32,
    pub(crate) varmap_index: u32,
    pub(crate) first_child_index: u32,
    pub(crate) last_child_index: u32,
    pub(crate) type_id: u16,    
}

impl Object {
    pub(crate) const INVALID_INDEX: u32 = u32::MAX;
}
