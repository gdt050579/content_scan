//! Procedural macros for the [`content_scan`] crate.
//!
//! This crate provides the `#[derive(ContentType)]` macro used to
//! automatically implement the `ContentType` trait for user-defined
//! `#[repr(u16)]` enums. End users normally import it through the
//! `content_scan` re-export rather than depending on this crate
//! directly.
//!
//! [`content_scan`]: https://docs.rs/content_scan

mod derive;
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
    match derive::process_content_type(input) {
        Ok(ts) => ts,
        Err(msg) => format!("compile_error!({:?});", msg).parse().unwrap(),
    }
}
