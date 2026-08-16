use std::collections::HashMap;

use crate::{ContentIdentifier, ContentType, IdentifyMethod, Matcher, MatcherBuilder};
use crate::utils;
pub(crate) struct IdentifierSet<T: ContentType> {
    identifiers: HashMap<u16, Box<dyn ContentIdentifier<T>>>,
    magics: Matcher<T>,
    extensions: Matcher<T>,
    names: Matcher<T>,
    no_prefilter_list: Vec<T>,
    temp_buffer: Vec<u8>,
}
impl<T: ContentType> IdentifierSet<T> {
    pub(crate) fn new(identifiers: Vec<(T, Box<dyn ContentIdentifier<T>>)>) -> Self {
        let mut map = HashMap::new();
        let mut magics = MatcherBuilder::new();
        let mut extensions = MatcherBuilder::new();
        let mut names = MatcherBuilder::new();
        let mut no_prefilter_list = Vec::new();
        for (content_type, identifier) in identifiers {
            if let Some(fast_id) = identifier.identify_method() {
                match fast_id {
                    IdentifyMethod::Magic(magic) => magics.add(content_type, magic),
                    IdentifyMethod::MultipleMagic(items) => {
                        for item in items {
                            magics.add(content_type, item);
                        }
                    }
                    IdentifyMethod::Extension(extension) => extensions.add(content_type, utils::ascii_lower_static(extension)),
                    IdentifyMethod::Extensions(items) => {
                        for item in items {
                            extensions.add(content_type, utils::ascii_lower_static(item));
                        }
                    }
                    IdentifyMethod::Name(name) => names.add(content_type, utils::ascii_lower_static(name)),
                    IdentifyMethod::Names(items) => {
                        for item in items {
                            names.add(content_type, utils::ascii_lower_static(item));
                        }
                    }
                }
            } else {
                no_prefilter_list.push(content_type);
            }
            map.insert(content_type.as_u16(), identifier);
        }
        Self {
            identifiers: map,
            magics: magics.build(),
            extensions: extensions.build(),
            names: names.build(),
            no_prefilter_list,
            temp_buffer: Vec::with_capacity(64),
        }
    }
    #[inline(always)]
    pub(crate) fn get(&self, content_type: T) -> Option<&Box<dyn ContentIdentifier<T>>> {
        self.identifiers.get(&content_type.as_u16())
    }
    #[inline(always)]
    pub(crate) fn type_from_file_name(&mut self, file_name: &[u8]) -> Option<T> {
        let b = if utils::contains_uppercase(file_name) {
            utils::copy_lowercase(file_name, &mut self.temp_buffer);
            self.temp_buffer.as_slice()
        } else {
            file_name
        };
        self.names.matches_exactly(b)
    }
    #[inline(always)]
    pub(crate) fn type_from_extension(&mut self, extension: &[u8]) -> Option<T> {
        let b = if utils::contains_uppercase(extension) {
            utils::copy_lowercase(extension, &mut self.temp_buffer);
            self.temp_buffer.as_slice()
        } else {
            extension
        };
        self.extensions.matches_exactly(b)
    }
    #[inline(always)]
    pub(crate) fn type_from_magic(&self, buffer: &[u8]) -> Option<T> {
        self.magics.starts_with(buffer)
    }
    /// Identifiers with no [`IdentifyMethod`](crate::IdentifyMethod), in
    /// registration order.
    #[inline(always)]
    pub(crate) fn identifiers_without_prefilter(&self) -> &[T] {
        &self.no_prefilter_list
    }
}
