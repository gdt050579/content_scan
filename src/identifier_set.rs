use std::collections::HashMap;

use crate::{ContentIdentifier, ContentType, FastID, Matcher, MatcherBuilder};

pub(crate) struct IdentifierSet<T: ContentType> {
    identifiers: HashMap<u16, Box<dyn ContentIdentifier<T>>>,
    magics: Matcher<T>,
    extensions: Matcher<T>,
    names: Matcher<T>,
}
impl<T: ContentType> IdentifierSet<T> {
    pub(crate) fn new(identifiers: Vec<(T, Box<dyn ContentIdentifier<T>>)>) -> Self {
        let mut map = HashMap::new();
        let mut magics = MatcherBuilder::new();
        let mut extensions = MatcherBuilder::new();
        let mut names = MatcherBuilder::new();
        for (content_type, identifier) in identifiers {
            if let Some(fast_id) = identifier.fast_id() {
                match fast_id {
                    FastID::Magic(magic) => magics.add(content_type, magic),
                    FastID::MultipleMagic(items) => {
                        for item in items {
                            magics.add(content_type, item);
                        }
                    }
                    FastID::Extension(extension) => extensions.add(content_type, extension.as_bytes()),
                    FastID::Extensions(items) => {
                        for item in items {
                            extensions.add(content_type, item.as_bytes());
                        }
                    }
                    FastID::Name(name) => names.add(content_type, name.as_bytes()),
                    FastID::Names(items) => {
                        for item in items {
                            names.add(content_type, item.as_bytes());
                        }
                    }
                }
            }
            map.insert(content_type.as_u16(), identifier);
        }
        Self {
            identifiers: map,
            magics: magics.build(),
            extensions: extensions.build(),
            names: names.build(),
        }
    }
    #[inline(always)]
    pub(crate) fn get(&self, content_type: T) -> Option<&Box<dyn ContentIdentifier<T>>> {
        self.identifiers.get(&content_type.as_u16())
    }
    #[inline(always)]
    pub(crate) fn type_from_file_name(&self, file_name: &[u8]) -> Option<T> {
        self.names.matches_exactly(file_name)
    }
    #[inline(always)]
    pub(crate) fn type_from_extension(&self, extension: &[u8]) -> Option<T> {
        self.extensions.matches_exactly(extension)
    }
    #[inline(always)]
    pub(crate) fn type_from_magic(&self, buffer: &[u8]) -> Option<T> {
        self.magics.starts_with(buffer)
    }
}
