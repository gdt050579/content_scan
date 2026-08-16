use super::packed_linear_list::Key;
use super::packed_linear_list::PackedLinearList;
use crate::ContentType;

pub(crate) struct FastMagicMatcher<T: ContentType> {
    two: Option<PackedLinearList<T, u32>>,
    three: Option<PackedLinearList<T, u32>>,
    four: Option<PackedLinearList<T, u32>>,
}
impl<T: ContentType> FastMagicMatcher<T> {
    pub(crate) fn new(patterns: &[(T, &'static [u8])]) -> Option<Self> {
        if patterns.is_empty() {
            return None;
        }
        let min_len = patterns.first().unwrap().1.len();
        let max_len = patterns.last().unwrap().1.len();
        if min_len < 2 || max_len > 4 {
            return None;
        }
        // patterns are sorted by length
        let mut two = None;
        let mut three = None;
        let mut four = None;
        // find start/end for size two
        let mut start = 0;
        let mut len = 0;
        for i in 0..patterns.len() {
            let pattern_len = patterns[i].1.len();
            if pattern_len != len {
                if len == 0 {
                    len = pattern_len;
                    start = i;
                    continue;
                }
                match len {
                    2 => two = PackedLinearList::new(&patterns[start..i]),
                    3 => three = PackedLinearList::new(&patterns[start..i]),
                    4 => four = PackedLinearList::new(&patterns[start..i]),
                    _ => return None,
                }
                len = pattern_len;
                start = i;
            }
        }
        match len {
            2 => two = PackedLinearList::new(&patterns[start..patterns.len()]),
            3 => three = PackedLinearList::new(&patterns[start..patterns.len()]),
            4 => four = PackedLinearList::new(&patterns[start..patterns.len()]),
            _ => return None,
        }
        Some(Self { two, three, four })
    }
    #[inline(always)]
    fn test_four(&self, data: &[u8]) -> Option<T> {
        if let Some(list) = self.four.as_ref() {
            if let Some(ct) = list.find(u32::pack(&data[..4])) {
                return Some(ct);
            }
        }
        None
    }
    #[inline(always)]
    fn test_three(&self, data: &[u8]) -> Option<T> {
        if let Some(list) = self.three.as_ref() {
            if let Some(ct) = list.find(u32::pack(&data[..3])) {
                return Some(ct);
            }
        }
        None
    }
    #[inline(always)]
    fn test_two(&self, data: &[u8]) -> Option<T> {
        if let Some(list) = self.two.as_ref() {
            if let Some(ct) = list.find(u32::pack(&data[..2])) {
                return Some(ct);
            }
        }
        None
    }
    #[inline(always)]
    pub(crate) fn starts_with(&self, data: &[u8]) -> Option<T> {
        if data.len() >= 4 {
            let result = self.test_four(data);
            if result.is_some() {
                return result;
            }
            let result = self.test_three(data);
            if result.is_some() {
                return result;
            }
            let result = self.test_two(data);
            if result.is_some() {
                return result;
            }
        } else if data.len() >= 3 {
            let result = self.test_three(data);
            if result.is_some() {
                return result;
            }
            let result = self.test_two(data);
            if result.is_some() {
                return result;
            }
        } else if data.len() >= 2 {
            let result = self.test_two(data);
            if result.is_some() {
                return result;
            }
        }
        None
    }
    #[inline(always)]
    pub(crate) fn matches_exactly(&self, data: &[u8]) -> Option<T> {
        match data.len() {
            2 => self.two.as_ref().and_then(|list| list.find(u32::pack(data))),
            3 => self.three.as_ref().and_then(|list| list.find(u32::pack(data))),
            4 => self.four.as_ref().and_then(|list| list.find(u32::pack(data))),
            _ => None,
        }
    }
}
