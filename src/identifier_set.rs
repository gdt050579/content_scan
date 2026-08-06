use std::collections::HashMap;

use crate::{ContentIdentifier, ContentType, FastID, Matcher, MatcherBuilder};

pub struct IdentifierSet<T: ContentType> {
    identifiers: HashMap<u16, Box<dyn ContentIdentifier<T>>>,
    magics: Matcher<T>,
    extensions: Matcher<T>,
    names: Matcher<T>,
}
impl<T: ContentType> IdentifierSet<T> {
    pub fn new(identifiers: Vec<(T, Box<dyn ContentIdentifier<T>>)>) -> Self {
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
                    },
                    FastID::Extension(extension) => extensions.add(content_type, extension.as_bytes()),
                    FastID::Extensions(items) => {
                        for item in items {
                            extensions.add(content_type, item.as_bytes());
                        }
                    },
                    FastID::Name(name) => names.add(content_type, name.as_bytes()),
                    FastID::Names(items) => {
                        for item in items {
                            names.add(content_type, item.as_bytes());
                        }
                    },
                }
            }
            map.insert(content_type.as_u16(), identifier);
        }
        Self { identifiers: map, magics: magics.build(), extensions: extensions.build(), names: names.build() }
    }
}