use crate::matcher::{Matcher, MatcherBuilder};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precedence {
    Lowest,
    Low,
    Medium,
    High,
    Highest,
}

enum FilterRule {
    IncludeExtensions(Matcher<bool>),
    ExcludeExtensions(Matcher<bool>),
    IncludeFileNames(Matcher<bool>),
    ExcludeFileNames(Matcher<bool>),
    Include(fn(&str, u64) -> bool),
    Exclude(fn(&str, u64) -> bool),
}

pub struct Filter {
    rules: Vec<FilterRule>,
    default_result: bool,
    check_extensions: bool,
    check_file_names: bool,
}
impl Filter {
    pub fn should_process(&self, path: &str, depth: u32, size: u64) -> bool {
        todo!()
    }
}

pub struct FilterBuilder {
    rules: Vec<(Precedence, FilterRule)>,
    default_result: bool,
}
impl FilterBuilder {
    pub fn new() -> Self {
        Self { rules: Vec::with_capacity(4), default_result: true }
    }
    pub fn include_extensions(mut self, prec: Precedence, extensions: &[&'static str]) -> Self {
        let mut matcher_builder = MatcherBuilder::new();
        for extension in extensions {
            matcher_builder.add(true, extension.as_bytes());
        }
        self.rules.push((prec, FilterRule::IncludeExtensions(matcher_builder.build())));
        self
    }
    pub fn exclude_extensions(mut self, prec: Precedence, extensions: &[&'static str]) -> Self {
        let mut matcher_builder = MatcherBuilder::new();
        for extension in extensions {
            matcher_builder.add(false, extension.as_bytes());
        }
        self.rules.push((prec, FilterRule::ExcludeExtensions(matcher_builder.build())));
        self
    }
    pub fn include_file_names(mut self, prec: Precedence, file_names: &[&'static str]) -> Self {
        let mut matcher_builder = MatcherBuilder::new();
        for file_name in file_names {
            matcher_builder.add(true, file_name.as_bytes());
        }
        self.rules.push((prec, FilterRule::IncludeFileNames(matcher_builder.build())));
        self
    }
    pub fn exclude_file_names(mut self, prec: Precedence, file_names: &[&'static str]) -> Self {
        let mut matcher_builder = MatcherBuilder::new();
        for file_name in file_names {
            matcher_builder.add(false, file_name.as_bytes());
        }
        self.rules.push((prec, FilterRule::ExcludeFileNames(matcher_builder.build())));
        self
    }
    pub fn include(mut self, prec: Precedence, callback: fn(&str, u64) -> bool) -> Self {
        self.rules.push((prec, FilterRule::Include(callback)));
        self
    }
    pub fn exclude(mut self, prec: Precedence, callback: fn(&str, u64) -> bool) -> Self {
        self.rules.push((prec, FilterRule::Exclude(callback)));
        self
    }
    pub fn deny_the_rest(mut self) -> ReadyFilterBuilder {
        self.default_result = false;
        ReadyFilterBuilder { builder: self }
    }
    pub fn allow_the_rest(mut self) -> ReadyFilterBuilder {
        self.default_result = true;
        ReadyFilterBuilder { builder: self }
    }
}
pub struct ReadyFilterBuilder {
    builder: FilterBuilder,
}
impl ReadyFilterBuilder {
    pub fn build(self) -> Filter {
        todo!()
    }
}
