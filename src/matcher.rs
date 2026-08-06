use super::ContentType;
use crate::trie::Trie;
pub enum Matcher<T: ContentType> {
    None,
    One { content_type: T, data: &'static [u8] },
    Trie(Trie)
}
impl<T: ContentType> Matcher<T> {
    pub(crate) fn starts_with(&self, content: &[u8]) -> Option<T> {
        if content.is_empty() {
            return None;
        }
        match self {
            Matcher::None => None,
            Matcher::One { content_type, data } => {
                if content.starts_with(data) {
                    Some(*content_type)
                } else {
                    None
                }
            },
            Matcher::Trie(trie) => {
                trie.starts_with(content)
            }
        }
    }
    pub(crate) fn matches_exactly(&self, content: &[u8]) -> Option<T> {
        if content.is_empty() {
            return None;
        }
        match self {
            Matcher::None => None,
            Matcher::One { content_type, data } => {
                if content.starts_with(data) {
                    Some(*content_type)
                } else {
                    None
                }
            },
            Matcher::Trie(trie) => {
                trie.matches_exactly(content)
            }
        }
    }
}
pub struct MatcherBuilder<T: ContentType> {
    data: Vec<(T, &'static [u8])>,
}
impl<T: ContentType> MatcherBuilder<T> {
    pub fn new() -> Self {
        Self { data: Vec::with_capacity(16) }
    }
    pub fn add(&mut self, content_type: T, data: &'static [u8]) {
        self.data.push((content_type, data));
    }
    pub fn build(self) -> Matcher<T> {
        todo!();
    }
}