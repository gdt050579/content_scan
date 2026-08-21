//! Procedural macros for the [`content_scan`] crate.
//!
//! This crate provides the `#[derive(ContentType)]` and
//! `#[derive(Dependencies)]` macros used to automatically implement
//! the corresponding traits. End users normally import them through
//! the `content_scan` re-export rather than depending on this crate
//! directly.
//!
//! [`content_scan`]: https://docs.rs/content_scan

mod content_type_derive;
mod dependencies_derive;
use proc_macro::*;
extern crate proc_macro;

/// Derives an implementation of `content_scan::ContentType` for a
/// `#[repr(u16)]` enum.
///
/// The derived implementation provides:
///
/// - `const COUNT: u16` — the number of variants of the enum.
/// - `fn as_u16(&self) -> u16` — the variant's discriminant.
/// - `fn from_u16(value: u16) -> Option<Self>` — the inverse mapping,
///   returning `None` for values that do not correspond to any
///   variant.
///
/// # Requirements
///
/// The target must be an enum whose variants carry no data and that
/// is annotated with `#[repr(u16)]`. All the usual derives expected
/// by the framework (`Debug`, `Copy`, `Clone`, `Eq`, `PartialEq`,
/// `Ord`, `PartialOrd`) should also be present.
///
/// # Example
///
/// ```ignore
/// use content_scan::ContentType;
///
/// #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ContentType)]
/// #[repr(u16)]
/// enum MyTypes {
///     Text,
///     Binary,
/// }
/// ```
#[proc_macro_derive(ContentType)]
pub fn derive_content_type(input: TokenStream) -> TokenStream {
    match content_type_derive::process(input) {
        Ok(ts) => ts,
        Err(msg) => format!("compile_error!({:?});", msg).parse().unwrap(),
    }
}

/// Derives an implementation of `content_scan::Dependencies`.
///
/// The type must be annotated with `#[Dependencies(...)]`. `name` is
/// required and must be a non-empty string. `requires` is optional
/// and may be a single string or an array of strings:
///
/// ```ignore
/// use content_scan::Dependencies;
///
/// #[derive(Dependencies)]
/// #[Dependencies(name = "xyz", requires = "abc")]
/// struct PluginA;
///
/// #[derive(Dependencies)]
/// #[Dependencies(name = "xyz", requires = ["abc", "123", "blablabla"])]
/// struct PluginB;
///
/// #[derive(Dependencies)]
/// #[Dependencies(name = "solo")]
/// struct PluginC;
/// ```
///
/// The derived methods exist only when `debug_assertions` are
/// enabled. In debug builds, `ScannerBuilder::build` uses `name` and
/// `requires` to check that required analyzers are registered with a
/// strictly smaller priority.
///
/// # Requirements
///
/// The target must be a non-generic `struct`, `enum`, or `union`.
/// Analyzers typically use this derive because
/// `content_scan::ContentAnalyzer` requires `Dependencies`.
#[proc_macro_derive(Dependencies, attributes(Dependencies))]
pub fn derive_dependencies(input: TokenStream) -> TokenStream {
    match dependencies_derive::process(input) {
        Ok(ts) => ts,
        Err(msg) => format!("compile_error!({:?});", msg).parse().unwrap(),
    }
}