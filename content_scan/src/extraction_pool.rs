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

/// Storage for the per-session state of a [`ContentExtractor`](crate::ContentExtractor).
///
/// A single extractor instance is shared by every object of its type,
/// so it cannot keep the cursor of an in-progress extraction in a
/// plain field: a nested or interleaved extraction would overwrite it.
/// `ExtractionPool` holds one `T` per live session and returns the
/// [`ExtractionHandle`] that identifies it, which the scanner then
/// passes back into `advance`, `extract`, and `release`.
///
/// Slots are recycled through a free list, and each handle carries a
/// monotonically increasing uid. A stale handle whose slot has already
/// been reused therefore resolves to `None` from [`get`](Self::get) /
/// [`get_mut`](Self::get_mut) instead of silently aliasing another
/// session's state.
///
/// ```ignore
/// #[derive(Default)]
/// struct MyExtractor {
///     pool: ExtractionPool<Cursor>,
/// }
///
/// impl ContentExtractor<MyTypes> for MyExtractor {
///     fn acquire(&mut self, _: &mut dyn Content<MyTypes>, _: &mut VarMap) -> Option<ExtractionHandle> {
///         Some(self.pool.acquire_slot(Cursor::default()))
///     }
///     fn advance(&mut self, handle: ExtractionHandle, _: &mut dyn Content<MyTypes>) -> Option<&Entry> {
///         let cursor = self.pool.get_mut(handle)?;
///         // ...advance the cursor...
///         self.pool.update_entry("child", size);
///         Some(self.pool.entry())
///     }
///     // ...
///     fn release(&mut self, handle: ExtractionHandle) {
///         self.pool.release_slot(handle);
///     }
/// }
/// ```
#[derive(Default)]
pub struct ExtractionPool<T> {
    pool: Vec<Option<(u32, T)>>,
    free_list: Vec<u32>,
    last_uid: u32,
}
impl<T> ExtractionPool<T> {
    /// Creates an empty pool with room for `capacity` concurrent
    /// sessions before it needs to grow.
    pub fn new(capacity: usize) -> Self {
        Self {
            pool: Vec::with_capacity(capacity),
            free_list: Vec::with_capacity(capacity),
            last_uid: 0,
        }
    }
    /// Stores `obj` in a free slot and returns the handle that
    /// identifies it.
    ///
    /// Call this from
    /// [`acquire`](crate::ContentExtractor::acquire) and return the
    /// handle to the scanner.
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
    /// Drops the state behind `handle` and returns its slot to the
    /// free list.
    ///
    /// Call this from
    /// [`release`](crate::ContentExtractor::release). Handles that no
    /// longer refer to a live session are ignored.
    pub fn release_slot(&mut self, handle: ExtractionHandle) {
        let idx = handle.index as usize;
        if idx >= self.pool.len() {
            return;
        }
        let index_checked = if let Some((id, _)) = &self.pool[idx] {
            *id == handle.uid
        } else {
            false
        };
        if index_checked {
            self.pool[idx] = None;
            self.free_list.push(handle.index);
        }
    }
    /// Borrows the state behind `handle`, or `None` if the handle no
    /// longer refers to a live session.
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
    /// Mutably borrows the state behind `handle`, or `None` if the
    /// handle no longer refers to a live session.
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
