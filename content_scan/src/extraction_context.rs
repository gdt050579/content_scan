use varmap::VarMap;

use crate::{ContentType, Context};

pub struct ExtractionContext<'a> {
    pub offset: u64,
    pub length: Option<u64>,
    pub params: &'a VarMap,
}

#[derive(Copy, Clone)]
pub(crate) struct ExtractionRequest<T: ContentType> {
    pub(crate) content_type: T,
    pub(crate) start: u64,
    pub(crate) len: Option<u64>,
    pub(crate) params_handle: Option<varmap::PoolHandle>,
}

// in context.rs (or its own module)

/// Fluent builder for an extraction request, returned by
/// [`Context::request_extract`].
///
/// Borrows the [`Context`] for the duration of the chain. `.param()`
/// lazily reserves a pooled [`VarMap`] on first use, so a param-less
/// request touches no map at all. Commit with [`emit`](Self::emit);
/// dropping the builder without emitting files nothing and returns any
/// reserved map to the pool.
#[must_use = "an extraction request does nothing until `.emit()` is called"]
pub struct ExtractRequestBuilder<'c, T: ContentType> {
    ctx: &'c mut Context<T>,
    request: ExtractionRequest<T>,
}

impl<'c, T: ContentType> ExtractRequestBuilder<'c, T> {
    pub(crate) fn new(ctx: &'c mut Context<T>, content_type: T) -> Self {
        Self {
            ctx,
            request: ExtractionRequest {
                content_type,
                start: 0,
                len: None,
                params_handle: None,
            },
        }
    }
    /// Sets the byte offset within the parent where extraction begins.
    ///
    /// Defaults to `0` (whole content) when not called.
    #[inline]
    pub fn at(mut self, start: u64) -> Self {
        self.request.start = start;
        self
    }

    /// Sets the length of the region.
    ///
    /// Absent means the extractor determines the extent itself (see the
    /// `len: None` contract). Calling `.len(n)` asserts the analyzer
    /// knows the region is `n` bytes.
    #[inline]
    pub fn len(mut self, len: u64) -> Self {
        self.request.len = Some(len);
        self
    }

    /// Adds one extractor-specific parameter. Repeatable.
    ///
    /// The first call reserves a pooled `VarMap`; subsequent calls
    /// write into the same map. A request with no `.param()` call
    /// carries no map.
    ///
    /// `V` is whatever your `VarMap` accepts as a value; adjust the
    /// bound / insert call to match its real API.
    #[inline]
    pub fn param<V>(mut self, key: varmap::Key, value: V) -> Self
    where
        V: varmap::VarMapValue,
    {
        let handle = match self.request.params_handle {
            Some(handle) => handle,
            None => {
                let handle = self.ctx.varmap_pool.allocate();
                self.request.params_handle = Some(handle);
                handle
            }
        };
        if let Some(vm) = self.ctx.varmap_pool.get_mut(handle) {
            vm.set(key, value);
        }
        self
    }

    /// Commits the request into the [`Context`]'s queue for the current
    /// object. After this the builder is consumed and files nothing
    /// further.
    #[inline]
    pub fn emit(mut self) {
        self.ctx.extraction_requests.push(self.request);
        self.request.params_handle = None;
    }
}

impl<'c, T: ContentType> Drop for ExtractRequestBuilder<'c, T> {
    fn drop(&mut self) {
        // If we reserved a param map but never emitted, give it back.
        if let Some(handle) = self.request.params_handle {
            self.ctx.varmap_pool.release(handle);
            self.request.params_handle = None;
        }
    }
}