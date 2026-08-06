use varmap::VarMap;

pub struct Context {
    pub(crate) global: VarMap,
    pub(crate) extract: VarMap,
    pub(crate) objects_scanned: u32,
}
impl Context {
    pub(crate) fn new() -> Self {
        Self {
            global: VarMap::new(),
            extract: VarMap::new(),
            objects_scanned: 0,
        }
    }
    pub(crate) fn clear(&mut self) {
        self.global.clear();
        self.extract.clear();
        self.objects_scanned = 0;
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
    #[inline(always)]
    pub fn objects_scanned(&self) -> u32 {
        self.objects_scanned
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
    pub fn objects_scanned(&self) -> u32 {
        self.context.objects_scanned
    }
}