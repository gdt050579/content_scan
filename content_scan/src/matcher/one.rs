use crate::ContentType;

pub(crate) struct OneMatcher<T: ContentType> {
    content_type: T,
    data: &'static [u8],
}
impl<T: ContentType> OneMatcher<T> {
    pub(crate) fn new(content_type: T, data: &'static [u8]) -> Self {
        Self { content_type, data }
    }
    #[inline(always)]
    pub(crate) fn starts_with(&self, data: &[u8]) -> Option<T> {
        if data.starts_with(self.data) {
            Some(self.content_type)
        } else {
            None
        }
    }
    #[inline(always)]
    pub(crate) fn matches_exactly(&self, data: &[u8]) -> Option<T> {
        if data == self.data {
            Some(self.content_type)
        } else {
            None
        }
    }
}
