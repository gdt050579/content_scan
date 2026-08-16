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

pub(super) struct ExtractorList<T, CT: ContentType> {
    plugins: Vec<(CT, T)>,
    fast_map: Vec<VecRange>,
}
impl<T, CT: ContentType> ExtractorList<T, CT> {
    pub(super) fn new(plugins: Vec<(CT, T)>) -> Self {
        let mut p = plugins;
        p.sort_by_key(|(ct, _)| *ct);
        let mut fast_map = Vec::with_capacity(CT::COUNT as usize);
        fast_map.resize(CT::COUNT as usize, VecRange::EMPTY);
        // populate fast_map
        let mut last_type_id = 0;
        let mut start_pos = u16::MAX;
        for (pos, (content_type, _)) in p.iter().enumerate() {
            let ctid = content_type.as_u16();
            if start_pos == u16::MAX {
                start_pos = pos as u16;
                last_type_id = ctid;
                continue;
            }
            if ctid != last_type_id {
                fast_map[last_type_id as usize] = VecRange::new(start_pos, pos as u16);
                last_type_id = ctid;
                start_pos = pos as u16;
                continue;
            }
        }
        if start_pos != u16::MAX {
            // at least one extractor for this type
            fast_map[last_type_id as usize] = VecRange::new(start_pos, p.len() as u16);
        }
        Self { plugins: p, fast_map }
    }
    #[inline(always)]
    pub(super) fn range(&self, content_type: CT) -> Option<(usize, usize)> {
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
    pub(super) unsafe fn get(&mut self, index: usize) -> &mut T {
        unsafe { &mut self.plugins.get_unchecked_mut(index).1 }
    }
    #[inline(always)]
    pub(super) fn len(&self) -> usize {
        self.plugins.len()
    }
}
