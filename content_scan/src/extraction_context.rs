use varmap::VarMap;

use crate::{ContentType, Context, FindingMetadata};

/// Region of a parent [`Content`](crate::Content) that a
/// [`ContentExtractor`](crate::ContentExtractor) should look at, plus
/// optional analyzer-supplied parameters.
///
/// Passed to
/// [`ContentExtractor::create_session`](crate::ContentExtractor::create_session).
/// Copy the fields you need into the [`ExtractionSession`](crate::ExtractionSession);
/// the context is only valid for that call.
///
/// When the scanner invokes an extractor because the parent was
/// identified as that extractor's type, the context covers the whole
/// object: `offset = 0`, `length = Some(content.size())`,
/// `params = None`. When an analyzer queued the pass with
/// [`Context::request_extract`], the fields come from that request.
pub struct ExtractionContext<'a> {
    /// First byte of the region within the parent content.
    pub offset: u64,
    /// Size of the region in bytes, when known.
    ///
    /// `Some(n)` asserts the region is `n` bytes. `None` means the
    /// extractor determines the extent itself (parse until the format
    /// ends, scan to EOF, …).
    pub length: Option<u64>,
    /// Analyzer-supplied extras (password, codec, flags, …).
    ///
    /// `None` when the request had no
    /// [`.param()`](ExtractRequestBuilder::param) calls (including
    /// type-specific extraction of the parent itself). `Some` borrows
    /// the pooled map for the duration of `create_session` only —
    /// copy values out if the session needs them later.
    pub params: Option<&'a VarMap>,
}

pub(crate) struct ExtractionRequestMetadata {
    pub(crate) start: u64,
    pub(crate) len: Option<u64>,
    pub(crate) params_handle: Option<varmap::PoolHandle>,
}

#[derive(Copy, Clone)]
pub(crate) struct ExtractionRequest<T: ContentType> {
    pub(crate) content_type: T,
    pub(crate) start: u64,
    pub(crate) len: Option<u64>,
    pub(crate) params_handle: Option<varmap::PoolHandle>,
}

/// Fluent builder for an extraction request, returned by
/// [`Context::request_extract`].
///
/// Borrows the [`Context`] for the duration of the chain. [`.param()`](Self::param)
/// lazily reserves a pooled [`VarMap`] on first use, so a param-less
/// request touches no map at all. Commit with [`emit`](Self::emit);
/// dropping the builder without emitting files nothing and returns any
/// reserved map to the pool.
///
/// ```ignore
/// context.request_extract(MyTypes::Zip)
///     .at(0x1000)
///     .len(4096)
///     .param(var!("password"), "secret")
///     .emit();
/// ```
#[must_use = "an extraction request does nothing until `.emit()` is called"]
pub struct ExtractRequestBuilder<'c, T: ContentType, M: FindingMetadata> {
    ctx: &'c mut Context<T, M>,
    request: ExtractionRequest<T>,
}

impl<'c, T: ContentType, M: FindingMetadata> ExtractRequestBuilder<'c, T, M> {
    pub(crate) fn new(ctx: &'c mut Context<T, M>, content_type: T) -> Self {
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
    /// Defaults to `0` (start of the parent) when not called.
    #[inline]
    pub fn at(mut self, start: u64) -> Self {
        self.request.start = start;
        self
    }

    /// Sets the length of the region in bytes.
    ///
    /// Omit this call to leave [`ExtractionContext::length`] as `None`,
    /// meaning the extractor determines the extent itself. Calling
    /// `.len(n)` asserts the analyzer knows the region is `n` bytes.
    #[inline]
    pub fn len(mut self, len: u64) -> Self {
        self.request.len = Some(len);
        self
    }

    /// Adds one extractor-specific parameter. Repeatable.
    ///
    /// The first call reserves a pooled [`VarMap`]; subsequent calls
    /// write into the same map. A request with no `.param()` call
    /// carries no map, and `create_session` sees
    /// [`ExtractionContext::params`] as `None`.
    ///
    /// `V` must implement [`varmap::VarMapValue`]. Keys are typically
    /// created with the `var!` macro re-exported from this crate.
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
        self.ctx.extraction_requests_stack.push(self.request);
        self.request.params_handle = None;
    }
}

impl<'c, T: ContentType, M: FindingMetadata> Drop for ExtractRequestBuilder<'c, T, M> {
    fn drop(&mut self) {
        // If we reserved a param map but never emitted, give it back.
        if let Some(handle) = self.request.params_handle {
            self.ctx.varmap_pool.release(handle);
            self.request.params_handle = None;
        }
    }
}
