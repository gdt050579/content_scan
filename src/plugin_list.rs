use super::ContentType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VecRange {
    start: u16,
    end: u16,
}
impl VecRange {
    const EMPTY: Self = Self { start: 0, end: 0 };
    fn new(start: u16, end: u16) -> Self {
        Self { start, end }
    }
    #[inline(always)]
    fn is_empty(&self) -> bool {
       self.end == 0
    }
}

pub(super) struct PluginsList<T> {
    plugins: Vec<(u32, T)>,
    fast_map: Vec<VecRange>,
    generic_range: Option<(usize, usize)>,
}
impl<T> PluginsList<T> {
    pub(super) fn new(plugins: Vec<(u32, T)>, max_count: u16) -> Self {
        let mut p = plugins;
        p.sort_by_key(|(hash, _)| *hash);
        let mut fast_map = Vec::with_capacity(max_count as usize);
        fast_map.resize(max_count as usize, VecRange::EMPTY);
        // populate fast_map
        let mut last_type_id = 0;
        let mut start_pos = u16::MAX;
        for (pos, (hash, _)) in p.iter().enumerate() {
            let type_id = (hash >> 16) as u16;
            if type_id >= max_count && type_id != 0xFFFF {
                panic!("Invalid type_id: {} (expecting a value between 0 and {})", type_id, max_count);
            }
            if start_pos == u16::MAX {
                start_pos = pos as u16;
                last_type_id = type_id;
                continue;
            }
            if type_id != last_type_id {
                fast_map[last_type_id as usize] = VecRange::new(start_pos as u16, pos as u16);
                last_type_id = type_id;
                start_pos = pos as u16;
                continue;
            }
        }
        if (last_type_id < max_count) && (start_pos != u16::MAX) {
            fast_map[last_type_id as usize] = VecRange::new(start_pos as u16, p.len() as u16);
        }
        let generic_range = if last_type_id == 0xFFFF {
            Some((start_pos as usize, p.len() as usize))
        } else {
            None
        };
        Self { plugins: p, fast_map, generic_range }
    }
    #[inline(always)]
    pub(super) fn range<CT: ContentType>(&self, content_type: CT) -> Option<(usize, usize)> {
        let index = content_type.as_u16() as usize;
        if let Some(range) = self.fast_map.get(index) {
            if range.is_empty() {
                None
            } else {
                Some((range.start as usize, range.end as usize))
            }
        } else {
            None
        }
    }
    #[inline(always)]
    pub(super) fn generic_range(&self) -> Option<(usize, usize)> {
        self.generic_range
    }
    #[inline(always)]
    pub(super) unsafe fn get(&mut self, index: usize) -> &mut T {
        unsafe {&mut self.plugins.get_unchecked_mut(index).1 }
    }
    #[inline(always)]
    pub(super) fn len(&self) -> usize {
        self.plugins.len()
    }
}
