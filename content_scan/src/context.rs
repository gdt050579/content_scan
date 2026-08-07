use std::marker::PhantomData;

use crate::BufferArena;
use crate::ContentType;
use crate::Object;
use varmap::VarMap;

pub struct Context {
    pub(crate) global: VarMap,
    pub(crate) extract: VarMap,
    pub(crate) objects: Vec<Object>,
    pub(crate) path_arena: BufferArena,
    pub(crate) varmap_pool: Vec<VarMap>,
    pub(crate) used_local_varmaps: u32,
    pub(crate) local_varmaps_index: u32,
}
impl Context {
    pub(crate) fn new() -> Self {
        Self {
            global: VarMap::new(),
            extract: VarMap::new(),
            objects: Vec::with_capacity(16),
            path_arena: BufferArena::new(),
            varmap_pool: Vec::with_capacity(16),
            used_local_varmaps: 0,
            local_varmaps_index: Object::INVALID_INDEX,
        }
    }
    pub(crate) fn clear(&mut self) {
        self.global.clear();
        self.extract.clear();
        self.objects.clear();
        self.path_arena.clear();
        self.varmap_pool.clear();
        self.used_local_varmaps = 0;
        self.local_varmaps_index = Object::INVALID_INDEX;
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
    pub fn local(&mut self) -> &mut VarMap {
        if self.local_varmaps_index == Object::INVALID_INDEX {
            if self.used_local_varmaps >= self.varmap_pool.len() as u32 {
                self.varmap_pool.push(VarMap::new());
                self.used_local_varmaps = self.varmap_pool.len() as u32;
                self.local_varmaps_index = self.used_local_varmaps - 1;
            } else {
                self.local_varmaps_index = self.used_local_varmaps;
                self.used_local_varmaps += 1;
                self.varmap_pool[self.local_varmaps_index as usize].clear();
            }
        }
        &mut self.varmap_pool[self.local_varmaps_index as usize]
    }
    #[inline(always)]
    pub fn objects_scanned(&self) -> u32 {
        self.objects.len() as u32
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ScanContentHandle {
    pub(crate) index: u32,
}
pub struct ScanResult<'a, T: ContentType> {
    pub(crate) context: &'a Context,
    _extra: PhantomData<T>,
}
impl<'a, T: ContentType> ScanResult<'a, T> {
    pub(crate) fn new(context: &'a Context) -> Self {
        Self {
            context,
            _extra: PhantomData,
        }
    }
    pub fn global(&self) -> &VarMap {
        &self.context.global
    }
    pub fn objects_scanned(&self) -> u32 {
        self.context.objects.len() as u32
    }
    pub fn initial(&self) -> Option<ScanContentHandle> {
        if self.context.objects.is_empty() {
            None
        } else {
            Some(ScanContentHandle { index: 0 })
        }
    }
    pub fn parent(&self, handle: ScanContentHandle) -> Option<ScanContentHandle> {
        let object = self.context.objects.get(handle.index as usize)?;
        if object.parent_index as usize >= self.context.objects.len() {
            None
        } else {
            Some(ScanContentHandle { index: object.parent_index })
        }
    }
    pub fn next_sibling(&self, handle: ScanContentHandle) -> Option<ScanContentHandle> {
        let object = self.context.objects.get(handle.index as usize)?;
        if object.sibling_index as usize >= self.context.objects.len() {
            None
        } else {
            Some(ScanContentHandle { index: object.sibling_index })
        }
    }
    pub fn child(&self, handle: ScanContentHandle) -> Option<ScanContentHandle> {
        let object = self.context.objects.get(handle.index as usize)?;
        if object.child_index as usize >= self.context.objects.len() {
            None
        } else {
            Some(ScanContentHandle { index: object.child_index })
        }
    }
    pub fn local(&self, handle: ScanContentHandle) -> Option<&VarMap> {
        let object = self.context.objects.get(handle.index as usize)?;
        if object.varmap_index as usize >= self.context.varmap_pool.len() {
            None
        } else {
            self.context.varmap_pool.get(object.varmap_index as usize)
        }
    }
    pub fn path(&self, handle: ScanContentHandle) -> Option<&str> {
        let object = self.context.objects.get(handle.index as usize)?;
        let path = self.context.path_arena.get(object.path)?;
        Some(unsafe { std::str::from_utf8_unchecked(path) })
    }
    pub fn content_type(&self, handle: ScanContentHandle) -> Option<T> {
        let object = self.context.objects.get(handle.index as usize)?;
        T::from_u16(object.type_id)
    }
}
