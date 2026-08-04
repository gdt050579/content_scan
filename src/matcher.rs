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