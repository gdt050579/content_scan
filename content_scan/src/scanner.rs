use super::{
    analyzer_list::AnalyzerList, extractor_list::ExtractorList, Content, ContentAnalyzer, ContentExtractor, ContentIdentifier, ContentType, Filter,
    NextAction,
};
use crate::OwnedContentPtr;
use crate::ExtractionRequest;
use crate::ExtractionRequestMetadata;
use crate::utils;
use crate::ExtractionContext;
use crate::IdentifierSet;
use crate::Object;
use crate::{ContentPtr, Context, ScanResult};
use std::collections::HashSet;

/// The engine that drives a scan.
///
/// A `Scanner` bundles together a set of
/// [`ContentIdentifier`](crate::ContentIdentifier)s,
/// [`ContentAnalyzer`](crate::ContentAnalyzer)s,
/// [`ContentExtractor`](crate::ContentExtractor)s, an optional
/// [`Filter`], and a maximum recursion depth. Build one with
/// [`ScannerBuilder`] and drive scans with [`Scanner::scan`].
///
/// Scanners are reusable: the internal [`Context`] is cleared at the
/// start of every [`scan`](Self::scan) call, so a single instance can
/// process many independent inputs sequentially.
pub struct Scanner<T: ContentType> {
    filter: Option<Filter>,
    identifiers: IdentifierSet<T>,
    analyzers: AnalyzerList<Box<dyn ContentAnalyzer<T>>>,
    extractors: ExtractorList<Box<dyn ContentExtractor<T>>, T>,
    context: Context<T>,
    max_depth: u32,
}
impl<T: ContentType> Scanner<T> {
    const EMPTY_VAR_MAP: varmap::VarMap = varmap::VarMap::new();
    /// Scans a single top-level [`Content`] and returns the results.
    ///
    /// The scanner:
    ///
    /// 1. Clears its internal [`Context`] so that no state leaks
    ///    between calls.
    /// 2. Applies the configured [`Filter`], if any, to the top-level
    ///    content — but only when `filter_root` is `true`. If the
    ///    filter rejects it, the returned [`ScanResult`] contains no
    ///    objects.
    /// 3. Recursively identifies, analyzes, and extracts nested
    ///    content up to the configured
    ///    [`max_depth`](ScannerBuilder::max_depth).
    ///
    /// Pass `filter_root = false` when the root is a container the
    /// filter was not written to accept, such as a
    /// [`FolderContent`](crate::FolderContent) scanned with a filter
    /// that only allows certain file extensions. Extracted children
    /// are filtered either way, unless their
    /// [`Entry`](crate::Entry) sets `skip_from_filtering`.
    ///
    /// The returned [`ScanResult`] borrows from `self` and stays
    /// valid until the next call on this scanner. Copy anything you
    /// need to keep out before starting another scan.
    pub fn scan<'a>(&'a mut self, content: &mut dyn Content<T>, filter_root: bool) -> ScanResult<'a, T> {
        self.context.clear();
        if filter_root {
            if let Some(filter) = self.filter.as_mut() {
                if !filter.should_process(content.path(), content.size()) {
                    return ScanResult::new(&self.context);
                }
            }
        }
        let cptr = ContentPtr::new(content);
        self.inner_scan(cptr, 1, Object::INVALID_INDEX);
        ScanResult::new(&self.context)
    }
    fn inner_scan(&mut self, content: ContentPtr<T>, depth: u32, parent_index: u32) -> NextAction {
        self.context.local_varmap_handle = None; // so that next time someone ask for a local varmap, it will get one from the context varmap_pool
        self.context.current_object_index = None;
        let start_req_count = self.context.extraction_requests_stack.len();
        let (ty, my_index) = self.create_object(content, parent_index);

        let mut response = self.run_analyzers(content, ty);
        if response == NextAction::Continue {
            response = self.run_extractors(content, ty, depth, my_index, start_req_count);
        }
        self.restore_extraction_request_stack(start_req_count);
        match response {
            NextAction::Continue | NextAction::Skip => NextAction::Continue,
            NextAction::Exit => NextAction::Exit,
        }
    }
    fn create_object(&mut self, content: ContentPtr<T>, parent_index: u32) -> (Option<T>, u32) {
        let ty = self.retrieve_content_type(content);

        let path_index = self.context.path_arena.alloc(content.as_ref().path().as_printable_string().as_bytes());
        let obj = Object {
            path: path_index,
            parent_index,
            next_sibling_index: Object::INVALID_INDEX,
            varmap_handle: None,
            first_child_index: Object::INVALID_INDEX,
            last_child_index: Object::INVALID_INDEX,
            type_id: if let Some(ty) = ty { ty.as_u16() } else { u16::MAX },
        };
        let my_index = self.context.objects.len() as u32;
        self.context.objects.push(obj);
        self.context.current_object_index = Some(my_index);
        // links to parent and sibling
        let mut last_sibling_index = Object::INVALID_INDEX;
        if parent_index != Object::INVALID_INDEX {
            if let Some(parent) = self.context.objects.get_mut(parent_index as usize) {
                if parent.first_child_index == Object::INVALID_INDEX {
                    parent.first_child_index = my_index;
                } else {
                    last_sibling_index = parent.last_child_index;
                }
                parent.last_child_index = my_index;
                //println!("Parent: {}, Me: {}, First: {}, Last: {} -> {}", parent_index, my_index, parent.first_child_index, parent.last_child_index, content.path());
            }
        }
        if last_sibling_index != Object::INVALID_INDEX {
            if let Some(last_sibling) = self.context.objects.get_mut(last_sibling_index as usize) {
                last_sibling.next_sibling_index = my_index;
            }
        }
        (ty, my_index)
    }
    fn run_analyzers(&mut self, content: ContentPtr<T>, content_type: Option<T>) -> NextAction {
        if let Some(ty) = content_type {
            if let Some((start, end)) = self.analyzers.range(ty) {
                let res = self.scan_range(content, start, end);
                if matches!(res, NextAction::Skip | NextAction::Exit) {
                    return res;
                }
            }
        }
        // generic analyzers
        if let Some((start, end)) = self.analyzers.generic_range() {
            let res = self.scan_range(content, start, end);
            if matches!(res, NextAction::Skip | NextAction::Exit) {
                return res;
            }
        }
        NextAction::Continue
    }
    fn run_extractors(
        &mut self,
        mut content: ContentPtr<T>,
        content_type: Option<T>,
        depth: u32,
        my_index: u32,
        start_req_index: usize,
    ) -> NextAction {
        // type-specific extractors
        if let Some(ty) = content_type {
            if let Some((start, end)) = self.extractors.range(ty) {
                let res = self.extract_range(content, start, end, depth, my_index, None);
                if matches!(res, NextAction::Skip | NextAction::Exit) {
                    return res;
                }
            }
        }
        // run extraction requests
        let req_count = self.context.extraction_requests_stack.len();
        for i in start_req_index..req_count {
            let ty = self.context.extraction_requests_stack[i].content_type;
            if let Some((start, end)) = self.extractors.range(ty) {
                let res = self.extract_range(content, start, end, depth, my_index, Some(i as u32));
                if matches!(res, NextAction::Skip | NextAction::Exit) {
                    return res;
                }
            } else {
                if let Some(handle) = self.context.extraction_requests_stack[i].params_handle {
                    self.context.varmap_pool.release(handle);
                    self.context.extraction_requests_stack[i].params_handle = None;
                }
            }
        }
        NextAction::Continue
    }
    fn restore_extraction_request_stack(&mut self, start_req_index: usize) {
        // first release the params
        for req in self.context.extraction_requests_stack.iter_mut().skip(start_req_index) {
            if let Some(handle) = req.params_handle {
                self.context.varmap_pool.release(handle);
            }
        }
        // restore the stack
        self.context.extraction_requests_stack.truncate(start_req_index);
    }
    fn scan_range(&mut self, mut content: ContentPtr<T>, start: usize, end: usize) -> NextAction {
        if (end <= start) || (end > self.analyzers.len()) {
            return NextAction::Continue;
        }
        for i in start..end {
            let result = unsafe { self.analyzers.get(i).analyze(content.as_mut(), &mut self.context) };
            match result {
                NextAction::Continue => continue,
                NextAction::Exit => return NextAction::Exit,
                NextAction::Skip => return NextAction::Skip,
            }
        }
        NextAction::Continue
    }
    fn extract_range(
        &mut self,
        content: ContentPtr<T>,
        start: usize,
        end: usize,
        depth: u32,
        parent_index: u32,
        req_index: Option<u32>,
    ) -> NextAction {
        if (end <= start) || (end > self.extractors.len()) {
            return NextAction::Continue;
        }
        let ec_metadata = if let Some(req_index) = req_index {
            let request = &self.context.extraction_requests_stack[req_index as usize];
            ExtractionRequestMetadata {
                start: request.start,
                len: request.len,
                params_handle: request.params_handle,
            }
        } else {
            ExtractionRequestMetadata {
                start: 0,
                len: Some(content.as_ref().size()),
                params_handle: None,
            }
        };
        let next_action = {
            let mut next_action = NextAction::Continue;          
            for i in start..end {
                let result = self.extract_content(content, i, depth, parent_index, &ec_metadata);
                match result {
                    NextAction::Continue => continue,
                    NextAction::Exit | NextAction::Skip => {
                        next_action = result;
                        break;
                    }
                }
            }
            next_action
        };
        if let Some(handle) = ec_metadata.params_handle {
            self.context.varmap_pool.release(handle);
            if let Some(req_index) = req_index {
                self.context.extraction_requests_stack[req_index as usize].params_handle = None;
            }
        }        
        next_action
    }
    fn extract_content(&mut self, content: ContentPtr<T>, index: usize, depth: u32, parent_index: u32, ec_metadata: &ExtractionRequestMetadata) -> NextAction {
        if depth >= self.max_depth {
            return NextAction::Continue;
        }
        let len = self.extractors.len();
        if index >= len {
            return NextAction::Continue;
        }
        let extractor = unsafe { self.extractors.get(index) };
        let ec = ExtractionContext {
            offset: ec_metadata.start,
            length: ec_metadata.len,
            params: ec_metadata.params_handle.and_then(|h| self.context.varmap_pool.get(h)),
        };
        let session = extractor.create_session(OwnedContentPtr::new(content), &ec);
        if let Some(mut session) = session {
            while let Some(entry) = session.advance() {
                if !entry.skip_from_filtering {
                    if let Some(filter) = self.filter.as_mut() {
                        if !filter.should_process(&entry.path, entry.size) {
                            // println!("Skip: {:?}", &entry.path);
                            continue;
                        }
                    }
                }
                if let Some(mut extracted_content) = session.extract() {
                    let c_ptr = ContentPtr::new(extracted_content.as_mut());
                    let result = self.inner_scan(c_ptr, depth + 1, parent_index);
                    match result {
                        NextAction::Continue => continue,
                        NextAction::Exit => return NextAction::Exit,
                        NextAction::Skip => return NextAction::Continue,
                    }
                }
            }
        }
        NextAction::Continue
    }
    fn retrieve_content_type(&mut self, mut content: ContentPtr<T>) -> Option<T> {
        if let Some(ty) = content.as_ref().content_type() {
            return Some(ty);
        }
        let p = content.as_ref().path().as_bytes();
        // type from file name
        let file_name = utils::get_file_name(p);
        let type_from_file_name = self.identifiers.type_from_file_name(file_name);
        let extension = utils::get_extension(file_name);
        let type_from_extension = self.identifiers.type_from_extension(extension);
        // type from magic
        let type_from_magic = {
            if let Some(buf) = content.as_mut().read(0, 16) {
                self.identifiers.type_from_magic(buf)
            } else {
                None
            }
        };
        if let Some(ty) = type_from_magic {
            if self.validate_content_type(content, ty) {
                return Some(ty);
            }
        }
        if let Some(ty) = type_from_file_name {
            if self.validate_content_type(content, ty) {
                return Some(ty);
            }
        }
        if let Some(ty) = type_from_extension {
            if self.validate_content_type(content, ty) {
                return Some(ty);
            }
        }
        for &ty in self.identifiers.identifiers_without_prefilter() {
            if self.validate_content_type(content, ty) {
                return Some(ty);
            }
        }
        None
    }
    #[inline(always)]
    fn validate_content_type(&self, mut content: ContentPtr<T>, content_type: T) -> bool {
        self.identifiers
            .get(content_type)
            .map(|identifier| identifier.validate(content.as_mut()))
            .unwrap_or(false)
    }
}
/// Fluent builder for [`Scanner`].
///
/// Register identifiers, analyzers, extractors, and (optionally) a
/// [`Filter`] with the `add_*` / [`filter`](Self::filter) methods,
/// then call [`build`](Self::build) to obtain a ready-to-use
/// [`Scanner`].
///
/// # Priorities
///
/// Analyzers are registered with a `priority` byte. Within the same
/// [`ContentType`] bucket (or within the generic bucket), they run in
/// **ascending** priority order — lower numbers first.
///
/// Extractors have no priority: multiple extractors for the same type
/// run in registration order.
///
/// # Generic vs. typed plugins
///
/// - `add_analyzer` registers a plugin against a specific
///   [`ContentType`]. It only runs when the scanner has positively
///   identified content of that type.
/// - `add_extractor` registers a plugin against a specific
///   [`ContentType`]. It runs when the current object is that type,
///   and also when an analyzer requested that type via
///   [`Context::request_extract`](crate::Context::request_extract).
/// - `add_generic_analyzer` registers an analyzer that runs for every
///   content object, regardless of type (including unidentified
///   content). Generic analyzers run after the type-specific ones.
pub struct ScannerBuilder<T: ContentType> {
    filter: Option<Filter>,
    analyzers: Vec<(u32, Box<dyn ContentAnalyzer<T>>)>,
    extractors: Vec<(T, Box<dyn ContentExtractor<T>>)>,
    identifiers: Vec<(T, Box<dyn ContentIdentifier<T>>)>,
    max_depth: u32,
}
impl<T: ContentType> ScannerBuilder<T> {
    /// Creates a new, empty builder with sensible defaults.
    ///
    /// The default maximum recursion depth is `8`. Change it with
    /// [`max_depth`](Self::max_depth).
    pub fn new() -> Self {
        Self {
            filter: None,
            analyzers: Vec::with_capacity(16),
            extractors: Vec::with_capacity(4),
            identifiers: Vec::with_capacity(4),
            max_depth: 8,
        }
    }

    /// Attaches a pre-built [`Filter`] to the scanner.
    ///
    /// The filter is consulted for every content object (top-level
    /// and extracted) before any plugin runs. If a filter was
    /// previously set, it is replaced.
    pub fn filter(mut self, filter: Filter) -> Self {
        self.filter = Some(filter);
        self
    }

    /// Registers an analyzer for a specific `content_type`.
    ///
    /// `priority` (`0..=255`) orders analyzers registered for the
    /// same type; lower values run first. Multiple analyzers for the
    /// same `(content_type, priority)` are allowed and their relative
    /// order is unspecified.
    pub fn add_analyzer<A>(mut self, content_type: T, priority: u8, analyzer: A) -> Self
    where
        A: ContentAnalyzer<T> + 'static,
    {
        let hash = (content_type.as_u16() as u32) << 16 | priority as u32;
        self.analyzers.push((hash, Box::new(analyzer)));
        self
    }

    /// Registers a generic analyzer that runs on every content object.
    ///
    /// Generic analyzers run after all type-specific analyzers for a
    /// given object. `priority` (`0..=255`) orders generic analyzers
    /// among themselves; lower values run first.
    pub fn add_generic_analyzer<A>(mut self, priority: u8, analyzer: A) -> Self
    where
        A: ContentAnalyzer<T> + 'static,
    {
        let hash = 0xFFFF0000 | priority as u32;
        self.analyzers.push((hash, Box::new(analyzer)));
        self
    }

    /// Registers an extractor for a specific `content_type`.
    ///
    /// The extractor runs when the scanner has identified content of
    /// that type, and also when an analyzer requested that type via
    /// [`Context::request_extract`](crate::Context::request_extract).
    /// Multiple extractors for the same type run in registration
    /// order. Extracted children are scanned recursively as long as
    /// [`max_depth`](Self::max_depth) allows.
    pub fn add_extractor<E>(mut self, content_type: T, extractor: E) -> Self
    where
        E: ContentExtractor<T> + 'static,
    {
        self.extractors.push((content_type, Box::new(extractor)));
        self
    }

    /// Registers an identifier for `content_type`.
    ///
    /// Only one identifier is allowed per content type; registering
    /// two identifiers for the same type will cause
    /// [`build`](Self::build) to panic. An [`IdentifyMethod::Magic`]
    /// or [`IdentifyMethod::MultipleMagic`] pattern longer than 16
    /// bytes also panics at build.
    pub fn add_identifier<I>(mut self, content_type: T, identifier: I) -> Self
    where
        I: ContentIdentifier<T> + 'static,
    {
        self.identifiers.push((content_type, Box::new(identifier)));
        self
    }

    /// Sets the maximum recursion depth for the scanner.
    ///
    /// The value is clamped to `1..=u32::MAX - 2`. The top-level
    /// content is at depth `1`; children extracted from it are at
    /// depth `2`, and so on. Extraction stops when the next child
    /// would exceed `max_depth`.
    pub fn max_depth(mut self, max_depth: u32) -> Self {
        self.max_depth = max_depth.clamp(1, u32::MAX - 2);
        self
    }

    fn check_unique_identifiers(&self) {
        let mut m = HashSet::new();
        for (content_type, _) in &self.identifiers {
            if m.contains(&content_type.as_u16()) {
                panic!(
                    "There can only be one identifier for type ! Type {:?} has multiple identifiers !",
                    content_type
                );
            }
            m.insert(content_type.as_u16());
        }
    }
    /// Consumes the builder and produces a ready-to-use [`Scanner`].
    ///
    /// # Panics
    ///
    /// Panics if two identifiers are registered for the same
    /// [`ContentType`], or if an [`IdentifyMethod::Magic`] /
    /// [`IdentifyMethod::MultipleMagic`] pattern is longer than 16
    /// bytes.
    pub fn build(self) -> Scanner<T> {
        self.check_unique_identifiers();
        let analyzers = AnalyzerList::new(self.analyzers, T::COUNT);
        let extractors = ExtractorList::new(self.extractors);
        let identifiers = IdentifierSet::new(self.identifiers);
        Scanner {
            filter: self.filter,
            identifiers,
            analyzers,
            extractors,
            context: Context::new(),
            max_depth: self.max_depth,
        }
    }
}
