use crate::Entry;

/// Opaque token identifying one extraction session on a
/// [`ContentExtractor`](crate::ContentExtractor).
///
/// The scanner obtains a handle from
/// [`ContentExtractor::acquire`](crate::ContentExtractor::acquire) and
/// passes it back into every subsequent
/// [`advance`](crate::ContentExtractor::advance) /
/// [`extract`](crate::ContentExtractor::extract) /
/// [`release`](crate::ContentExtractor::release) call for that session.
/// This lets a single extractor instance drive several concurrent
/// (or nested) extractions by keying per-session state off the handle.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ExtractionHandle {
    index: u32,
    uid: u32,
}

impl ExtractionHandle {
    #[cfg(test)]
    pub(crate) const fn new(index: u32, uid: u32) -> Self {
        Self { index, uid }
    }
    #[cfg(test)]
    pub(crate) fn index(&self) -> u32 {
        self.index
    }
    #[cfg(test)]
    pub(crate) fn uid(&self) -> u32 {
        self.uid
    }
}

#[derive(Default)]
pub struct ExtractionPool<T> {
    entry: Entry,
    pool: Vec<Option<(u32, T)>>,
    free_list: Vec<u32>,
    last_uid: u32,
}
impl<T> ExtractionPool<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            entry: Entry::default(),
            pool: Vec::with_capacity(capacity),
            free_list: Vec::with_capacity(capacity),
            last_uid: 0,
        }
    }
    pub fn acquire_slot(&mut self, obj: T) -> ExtractionHandle {
        self.last_uid += 1;
        // try to get a free index
        let idx = if let Some(idx) = self.free_list.pop() {
            self.pool[idx as usize] = Some((self.last_uid, obj));
            idx
        } else {
            let idx = self.pool.len();
            self.pool.push(Some((self.last_uid, obj)));
            idx as u32
        };
        ExtractionHandle { index: idx, uid: self.last_uid }
    }
    pub fn release_slot(&mut self, handle: ExtractionHandle) {
        let idx = handle.index as usize;
        if idx >= self.pool.len() {
            return;
        }
        let index_checked = if let Some((id, _)) = &self.pool[idx] {
            if *id == handle.uid {
                true
            } else {
                false
            }
        } else {
            false
        };
        if index_checked {
            self.pool[idx] = None;
            self.free_list.push(handle.index);
        }
    }
    #[inline(always)]
    pub fn entry(&self) -> &Entry {
        &self.entry
    }
    #[inline(always)]
    pub fn update_entry(&mut self, path: &str, size: u64) {
        self.entry.path.clear();
        self.entry.path.push_str(path);
        self.entry.size = size;
    }
    pub fn get(&self, handle: ExtractionHandle) -> Option<&T> {
        let idx = handle.index as usize;
        if idx >= self.pool.len() {
            return None;
        }
        if let Some((id, value)) = &self.pool[idx] {
            if *id == handle.uid {
                Some(value)
            } else {
                None
            }
        } else {
            None
        }
    }
    pub fn get_mut(&mut self, handle: ExtractionHandle) -> Option<&mut T> {
        let idx = handle.index as usize;
        if idx >= self.pool.len() {
            return None;
        }
        if let Some((id, value)) = &mut self.pool[idx] {
            if *id == handle.uid {
                Some(value)
            } else {
                None
            }
        } else {
            None
        }
    }
}
