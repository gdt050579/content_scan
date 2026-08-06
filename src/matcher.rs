mod one;
mod trie;

#[cfg(test)]
mod tests;

use one::OneMatcher;
use trie::{Trie, TrieBuilder};

use super::ContentType;
pub enum Matcher<T: ContentType> {
    None,
    One(OneMatcher<T>),
    Trie(Trie),
}
impl<T: ContentType> Matcher<T> {
    pub(crate) fn starts_with(&self, content: &[u8]) -> Option<T> {
        if content.is_empty() {
            return None;
        }
        match self {
            Matcher::None => None,
            Matcher::One(one) => one.starts_with(content),
            Matcher::Trie(trie) => trie.starts_with(content).map(|value| T::from_u16(value).expect("Invalid content type")),
        }
    }
    pub(crate) fn matches_exactly(&self, content: &[u8]) -> Option<T> {
        if content.is_empty() {
            return None;
        }
        match self {
            Matcher::None => None,
            Matcher::One(one) => one.matches_exactly(content),
            Matcher::Trie(trie) => trie
                .matches_exactly(content)
                .map(|value| T::from_u16(value).expect("Invalid content type")),
        }
    }
}
pub struct MatcherBuilder<T: ContentType> {
    data: Vec<(T, &'static [u8])>,
}
impl<T: ContentType> MatcherBuilder<T> {
    pub fn new() -> Self {
        Self {
            data: Vec::with_capacity(16),
        }
    }
    pub fn add(&mut self, content_type: T, data: &'static [u8]) {
        self.data.push((content_type, data));
    }
    pub fn build(self) -> Matcher<T> {
        todo!();
    }
}
