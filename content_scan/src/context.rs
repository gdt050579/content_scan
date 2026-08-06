use varmap::VarMap;

pub struct Context {
    pub(crate) global: VarMap,
    pub(crate) extract: VarMap,
}
impl Context {
    pub(crate) fn new() -> Self {
        Self {
            global: VarMap::new(),
            extract: VarMap::new(),
        }
    }
    pub(crate) fn clear(&mut self) {
        self.global.clear();
        self.extract.clear();
    }
    pub(crate) fn clear_extract(&mut self) {
        self.extract.clear();
    }
    #[inline(always)]
    pub fn global(&mut self) -> &mut VarMap {
        &mut self.global
    }
    #[inline(always)]
    pub fn extract(&mut self) -> &mut VarMap {
        &mut self.extract
    }
}

pub struct ScanResult<'a> {
    pub(crate) context: &'a Context,
}
impl<'a> ScanResult<'a> {
    pub(crate) fn new(context: &'a Context) -> Self {
        Self { context }
    }
    pub fn global(&self) -> &VarMap {
        &self.context.global
    }
}