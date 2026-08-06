use super::ContentType;
pub struct Matcher<T: ContentType> {
    a: T,
}
impl<T: ContentType> Matcher<T> {
    pub(crate) fn starts_with(&self, content: &[u8]) -> Option<T> {
        todo!();
    }
    pub(crate) fn matches_exactly(&self, content: &[u8]) -> Option<T> {
        todo!();
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