use varmap::VarMap;

pub struct ExtractionContext<'a> {
    pub offset: u64,
    pub length: Option<u64>,
    pub params: &'a VarMap,
}