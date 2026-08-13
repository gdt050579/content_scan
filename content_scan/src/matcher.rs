mod one;
mod trie;
mod packed_linear_list;
mod fast_magic;

#[cfg(test)]
mod tests;

use one::OneMatcher;
use trie::{Trie, TrieBuilder};
use fast_magic::FastMagicMatcher;

use super::ContentType;
pub enum Matcher<T: ContentType> {
    None,
    One(OneMatcher<T>),
    FastMagic(FastMagicMatcher<T>),
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
            Matcher::FastMagic(fast_magic) => fast_magic.starts_with(content),
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
            Matcher::FastMagic(fast_magic) => fast_magic.matches_exactly(content),
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
    pub fn build(mut self) -> Matcher<T> {
        self.data.sort_by_key(|(_, data)| data.len());
        if self.data.is_empty() {
            return Matcher::None;
        }
        if self.data.len() == 1 {
            return Matcher::One(OneMatcher::new(self.data[0].0, self.data[0].1));
        }
        if self.data.len() < 16 {
            // verific daca toate sunt sub 4 bytes
            if self.data.iter().all(|(_, data)| data.len() <= 4 && data.len() >= 2) {
                return Matcher::FastMagic(FastMagicMatcher::new(&self.data).expect("Invalid fast magic matcher !"));
            }
        }
        // altfel fac un trie
        let mut trie_builder = TrieBuilder::new();
        for (ct, data) in self.data {
            trie_builder.add(data, ct.as_u16());
        }
        Matcher::Trie(trie_builder.build())
    }
}
