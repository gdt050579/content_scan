//! # content_scan
//!
//! A pluggable, hierarchical content-scanning framework.
//!
//! `content_scan` provides the building blocks for walking arbitrary
//! content (files, buffers, nested archives, embedded resources, etc.),
//! identifying its type, analyzing it, and recursively extracting inner
//! content. It is intentionally transport-agnostic: the caller supplies
//! anything that implements [`Content`] and the [`Scanner`] takes care of
//! dispatch, recursion, filtering, and result aggregation.
//!
//! ## Concepts
//!
//! - **[`ContentType`]** – a user-defined enum (typically derived with
//!   `#[derive(ContentType)]`) that enumerates the content kinds your
//!   application understands.
//! - **[`Content`]** – a trait describing a byte-addressable piece of
//!   content with a path and a size. Use [`BufferContent`] for in-memory
//!   data, [`FileContent`] for a file on disk, [`FolderContent`] for a
//!   directory, or implement it for your own sources.
//! - **[`ContentIdentifier`]** – classifies a piece of content into a
//!   `ContentType` (via magic bytes, file name, or extension) and
//!   validates the guess.
//! - **[`ContentAnalyzer`]** – inspects content and records information
//!   into the [`Context`] (global, per-extraction, or per-object
//!   variable maps).
//! - **[`ContentExtractor`]** – produces child [`Content`] items from a
//!   parent (e.g. entries of an archive). Extracted children are scanned
//!   recursively up to a configurable depth. Per-session state lives in
//!   an [`ExtractionPool`], keyed by an [`ExtractionHandle`].
//!   [`FolderExtractor`] is a ready-made implementation that walks a
//!   directory.
//! - **[`Filter`]** – optional inclusion/exclusion rules applied before
//!   any plugin runs on a piece of content.
//! - **[`Scanner`] / [`ScannerBuilder`]** – wires all plugins together
//!   and drives a scan. The scan produces a [`ScanResult`] that can be
//!   navigated as a tree of [`ScanContentHandle`]s.
//!
//! ## Example
//!
//! ```ignore
//! use content_scan::*;
//!
//! #[derive(Debug, Copy, Clone, Eq, PartialEq, ContentType)]
//! #[repr(u16)]
//! enum MyTypes { Text }
//!
//! struct TextId;
//! impl ContentIdentifier<MyTypes> for TextId {
//!     fn identify_method(&self) -> Option<IdentifyMethod> {
//!         Some(IdentifyMethod::Extension("txt"))
//!     }
//!     fn validate(&self, _: &dyn Content<MyTypes>) -> bool { true }
//! }
//!
//! let mut scanner = ScannerBuilder::<MyTypes>::new()
//!     .add_identifier(MyTypes::Text, TextId)
//!     .build();
//!
//! let mut content = BufferContent::<MyTypes>::new(b"hello", "hello.txt");
//! let result = scanner.scan(&mut content, true);
//! assert_eq!(result.objects_scanned(), 1);
//! ```
//!
//! ## Scanning a directory
//!
//! ```ignore
//! use content_scan::*;
//!
//! # #[derive(Debug, Copy, Clone, Eq, PartialEq, ContentType)]
//! # #[repr(u16)]
//! # enum MyTypes { Text, Folder }
//! let mut scanner = ScannerBuilder::<MyTypes>::new()
//!     .add_extractor(MyTypes::Folder, 0, FolderExtractor::<MyTypes>::new(true))
//!     .build();
//!
//! let mut root = FolderContent::<MyTypes>::with_content_type("./src", MyTypes::Folder);
//! // `false`: the root folder itself is not tested against the filter
//! let result = scanner.scan(&mut root, false);
//! ```

mod plugin_list;
mod interfaces;
mod matcher;
mod content;
mod scanner;
mod filter;
mod identifier_set;
mod utils;
mod context;
mod object;
mod buffer_arena;
mod extraction_pool;
mod content_path;
#[cfg(test)]
mod tests;


pub use content::*;
pub use interfaces::*;
pub use filter::*;

use identifier_set::*;
use matcher::*;
use object::Object;
use buffer_arena::BufferArena;

pub use scanner::Scanner;
pub use scanner::ScannerBuilder;
pub use content_scan_proc_macro::ContentType;
pub use context::Context;
pub use context::ScanResult;
pub use context::ScanContentHandle;
pub use varmap::*;
pub use extraction_pool::ExtractionHandle;
pub use extraction_pool::ExtractionPool;
pub use content_path::*;

