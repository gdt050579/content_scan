use crate::matcher::{Matcher, MatcherBuilder};
use crate::utils;
use crate::ContentPath;

/// Precedence of a [`Filter`] rule.
///
/// When a filter is built the rules are grouped by their precedence
/// and evaluated from `Highest` to `Lowest`. Within the same
/// precedence bucket, rules are evaluated in the order they were
/// added.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precedence {
    /// Evaluated last, after every other precedence tier.
    Lowest,
    /// Evaluated after `Medium` but before `Lowest`.
    Low,
    /// Default middle tier.
    Medium,
    /// Evaluated after `Highest` but before `Medium`.
    High,
    /// Evaluated first, before every other precedence tier.
    Highest,
}

impl Precedence {
    fn rank(self) -> u8 {
        match self {
            Self::Lowest => 0,
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 3,
            Self::Highest => 4,
        }
    }
}

enum FilterRule {
    IncludeExtensions(Matcher<bool>),
    ExcludeExtensions(Matcher<bool>),
    IncludeFileNames(Matcher<bool>),
    ExcludeFileNames(Matcher<bool>),
    Include(fn(&ContentPath, u64) -> bool),
    Exclude(fn(&ContentPath, u64) -> bool),
}

/// Compiled inclusion/exclusion policy applied to every content item
/// before it is scanned.
///
/// A `Filter` is built with [`FilterBuilder`] and then handed to a
/// [`ScannerBuilder`](crate::ScannerBuilder) via
/// [`ScannerBuilder::filter`](crate::ScannerBuilder::filter). The
/// scanner consults it for the top-level content when
/// [`Scanner::scan`](crate::Scanner::scan) is called with
/// `filter_root = true`, and for every [`Entry`](crate::Entry) an
/// extractor emits (unless that entry sets
/// [`skip_from_filtering`](crate::Entry::skip_from_filtering)).
///
/// Extension and file-name rules are ASCII case-insensitive: both the
/// registered patterns and the path basename / extension are compared
/// in lowercase. `Photo.JPG` matches a filter that allows `jpg`.
pub struct Filter {
    rules: Vec<FilterRule>,
    default_result: bool,
    check_extensions: bool,
    check_file_names: bool,
    temp_ext: Vec<u8>,
    temp_filename: Vec<u8>,
}
impl Filter {
    pub(crate) fn should_process(&mut self, path: &ContentPath, size: u64) -> bool {
        let file_name = if self.check_file_names {
            let res = utils::get_file_name(path.as_bytes());
            if utils::contains_uppercase(res) {
                utils::copy_lowercase(res, &mut self.temp_filename)
            } else {
                res
            }
        } else {
            b""
        };
        let ext = if self.check_extensions {
            let res = utils::get_extension(file_name);
            if utils::contains_uppercase(res) {
                utils::copy_lowercase(res, &mut self.temp_ext)
            } else {
                res
            }
        } else {
            b""
        };

        for rule in &self.rules {
            match rule {
                FilterRule::IncludeExtensions(matcher) | FilterRule::ExcludeExtensions(matcher) => {
                    if let Some(res) = matcher.matches_exactly(ext) {
                        return res;
                    }
                }
                FilterRule::IncludeFileNames(matcher) | FilterRule::ExcludeFileNames(matcher) => {
                    if let Some(res) = matcher.matches_exactly(file_name) {
                        return res;
                    }
                }
                FilterRule::Include(cb) => {
                    if cb(path, size) {
                        return true;
                    }
                }
                FilterRule::Exclude(cb) => {
                    if cb(path, size) {
                        return false;
                    }
                }
            };
        }
        self.default_result
    }
}

/// Builder for a [`Filter`].
///
/// Rules are added with the `include_*` / `exclude_*` methods. Each
/// rule carries a [`Precedence`] that controls the order in which it
/// is evaluated. When you are done, terminate the builder with either
/// [`deny_the_rest`](Self::deny_the_rest) or
/// [`allow_the_rest`](Self::allow_the_rest) to obtain a
/// [`ReadyFilterBuilder`] and finally call
/// [`ReadyFilterBuilder::build`] to produce the [`Filter`].
pub struct FilterBuilder {
    rules: Vec<(Precedence, FilterRule)>,
    default_result: bool,
}
impl FilterBuilder {
    /// Creates a new, empty filter builder.
    ///
    /// Until at least one rule is added, the resulting filter will
    /// simply return the default outcome for every input.
    pub fn new() -> Self {
        Self {
            rules: Vec::with_capacity(4),
            default_result: true,
        }
    }

    /// Accepts content whose extension is one of `extensions` (ASCII
    /// case-insensitive, without the leading dot).
    pub fn include_extensions(mut self, prec: Precedence, extensions: &[&'static str]) -> Self {
        let mut matcher_builder = MatcherBuilder::new();
        for extension in extensions {
            matcher_builder.add(true, utils::ascii_lower_static(extension));
        }
        self.rules.push((prec, FilterRule::IncludeExtensions(matcher_builder.build())));
        self
    }

    /// Rejects content whose extension is one of `extensions` (ASCII
    /// case-insensitive, without the leading dot).
    pub fn exclude_extensions(mut self, prec: Precedence, extensions: &[&'static str]) -> Self {
        let mut matcher_builder = MatcherBuilder::new();
        for extension in extensions {
            matcher_builder.add(false, utils::ascii_lower_static(extension));
        }
        self.rules.push((prec, FilterRule::ExcludeExtensions(matcher_builder.build())));
        self
    }

    /// Accepts content whose file name (basename) is one of
    /// `file_names` (ASCII case-insensitive).
    pub fn include_file_names(mut self, prec: Precedence, file_names: &[&'static str]) -> Self {
        let mut matcher_builder = MatcherBuilder::new();
        for file_name in file_names {
            matcher_builder.add(true, utils::ascii_lower_static(file_name));
        }
        self.rules.push((prec, FilterRule::IncludeFileNames(matcher_builder.build())));
        self
    }

    /// Rejects content whose file name (basename) is one of
    /// `file_names` (ASCII case-insensitive).
    pub fn exclude_file_names(mut self, prec: Precedence, file_names: &[&'static str]) -> Self {
        let mut matcher_builder = MatcherBuilder::new();
        for file_name in file_names {
            matcher_builder.add(false, utils::ascii_lower_static(file_name));
        }
        self.rules.push((prec, FilterRule::ExcludeFileNames(matcher_builder.build())));
        self
    }

    /// Adds a custom inclusion callback.
    ///
    /// `callback` receives the [`ContentPath`] and the size of a
    /// content item and should return `true` to include it. Returning
    /// `false` simply lets the next rule decide (it is *not* an
    /// exclusion). Use [`ContentPath::as_printable_string`] to inspect
    /// the UTF-8 view, or [`ContentPath::as_path`] for a filesystem
    /// path.
    pub fn include(mut self, prec: Precedence, callback: fn(&ContentPath, u64) -> bool) -> Self {
        self.rules.push((prec, FilterRule::Include(callback)));
        self
    }

    /// Adds a custom exclusion callback.
    ///
    /// `callback` receives the [`ContentPath`] and the size of a
    /// content item and should return `true` to reject it. Returning
    /// `false` simply lets the next rule decide (it is *not* an
    /// inclusion). Use [`ContentPath::as_printable_string`] to inspect
    /// the UTF-8 view, or [`ContentPath::as_path`] for a filesystem
    /// path.
    pub fn exclude(mut self, prec: Precedence, callback: fn(&ContentPath, u64) -> bool) -> Self {
        self.rules.push((prec, FilterRule::Exclude(callback)));
        self
    }

    /// Terminates the builder with a *deny-by-default* policy.
    ///
    /// Content that does not match any rule will be rejected.
    pub fn deny_the_rest(mut self) -> ReadyFilterBuilder {
        self.default_result = false;
        ReadyFilterBuilder { builder: self }
    }

    /// Terminates the builder with an *allow-by-default* policy.
    ///
    /// Content that does not match any rule will be accepted.
    pub fn allow_the_rest(mut self) -> ReadyFilterBuilder {
        self.default_result = true;
        ReadyFilterBuilder { builder: self }
    }
}

impl Default for FilterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Finalized [`FilterBuilder`] ready to produce a [`Filter`].
///
/// This intermediate type exists to force the caller to explicitly
/// choose between [`FilterBuilder::deny_the_rest`] and
/// [`FilterBuilder::allow_the_rest`] before building the filter,
/// which makes the default behavior visible at every call site.
pub struct ReadyFilterBuilder {
    builder: FilterBuilder,
}
impl ReadyFilterBuilder {
    /// Consumes the builder and produces the compiled [`Filter`].
    ///
    /// Rules are compiled into efficient matchers (tries / magic
    /// tables) so evaluating the filter is cheap even when many
    /// patterns are registered. They are then ordered from
    /// [`Precedence::Highest`] to [`Precedence::Lowest`]; rules that
    /// share a precedence keep the order they were added.
    pub fn build(self) -> Filter {
        let check_extensions = self
            .builder
            .rules
            .iter()
            .any(|(_, rule)| matches!(rule, FilterRule::IncludeExtensions(_) | FilterRule::ExcludeExtensions(_)));
        let check_file_names = self
            .builder
            .rules
            .iter()
            .any(|(_, rule)| matches!(rule, FilterRule::IncludeFileNames(_) | FilterRule::ExcludeFileNames(_)));
        let mut ranked = self.builder.rules;
        ranked.sort_by_key(|b| std::cmp::Reverse(b.0.rank()));
        let rules = ranked.into_iter().map(|(_, rule)| rule).collect();
        Filter {
            rules,
            default_result: self.builder.default_result,
            check_extensions,
            check_file_names: check_file_names || check_extensions, // if we check extensions, we also check file names
            temp_ext: Vec::with_capacity(16),
            temp_filename: Vec::with_capacity(64),
        }
    }
}
