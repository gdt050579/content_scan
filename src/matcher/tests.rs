mod trie {
    use super::super::trie::TrieBuilder;

    #[test]
    fn empty_trie_returns_none() {
        let trie = TrieBuilder::new().build();
        assert_eq!(trie.starts_with(b""), None);
        assert_eq!(trie.starts_with(b"abc"), None);
        assert_eq!(trie.matches_exactly(b""), None);
        assert_eq!(trie.matches_exactly(b"abc"), None);
    }

    #[test]
    fn exact_match() {
        let mut builder = TrieBuilder::new();
        builder.add(b"hello", 42);
        let trie = builder.build();

        assert_eq!(trie.starts_with(b"hello"), Some(42));
        assert_eq!(trie.matches_exactly(b"hello"), Some(42));

        assert_eq!(trie.starts_with(b"hell"), None);
        assert_eq!(trie.matches_exactly(b"hell"), None);

        // Extra bytes: prefix match keeps last value; exact match rejects.
        assert_eq!(trie.starts_with(b"hello!"), Some(42));
        assert_eq!(trie.matches_exactly(b"hello!"), None);

        assert_eq!(trie.starts_with(b"world"), None);
        assert_eq!(trie.matches_exactly(b"world"), None);
    }

    #[test]
    fn multiple_distinct_words() {
        let mut builder = TrieBuilder::new();
        builder.add(b"cat", 1);
        builder.add(b"dog", 2);
        builder.add(b"bird", 3);
        let trie = builder.build();

        assert_eq!(trie.starts_with(b"cat"), Some(1));
        assert_eq!(trie.starts_with(b"dog"), Some(2));
        assert_eq!(trie.starts_with(b"bird"), Some(3));
        assert_eq!(trie.starts_with(b"cow"), None);

        assert_eq!(trie.matches_exactly(b"cat"), Some(1));
        assert_eq!(trie.matches_exactly(b"dog"), Some(2));
        assert_eq!(trie.matches_exactly(b"bird"), Some(3));
        assert_eq!(trie.matches_exactly(b"cow"), None);
    }

    #[test]
    fn shared_prefix() {
        let mut builder = TrieBuilder::new();
        builder.add(b"app", 10);
        builder.add(b"apple", 20);
        builder.add(b"apply", 30);
        let trie = builder.build();

        assert_eq!(trie.starts_with(b"app"), Some(10));
        assert_eq!(trie.starts_with(b"apple"), Some(20));
        assert_eq!(trie.starts_with(b"apply"), Some(30));
        // Continues past "app" into unmatched bytes; last match wins.
        assert_eq!(trie.starts_with(b"application"), Some(10));

        assert_eq!(trie.matches_exactly(b"app"), Some(10));
        assert_eq!(trie.matches_exactly(b"apple"), Some(20));
        assert_eq!(trie.matches_exactly(b"apply"), Some(30));
        assert_eq!(trie.matches_exactly(b"application"), None);
    }

    #[test]
    fn longest_matching_prefix_value() {
        let mut builder = TrieBuilder::new();
        builder.add(b"a", 1);
        builder.add(b"ab", 2);
        builder.add(b"abc", 3);
        let trie = builder.build();

        assert_eq!(trie.starts_with(b"a"), Some(1));
        assert_eq!(trie.starts_with(b"ab"), Some(2));
        assert_eq!(trie.starts_with(b"abc"), Some(3));
        assert_eq!(trie.starts_with(b"abcd"), Some(3));
        assert_eq!(trie.starts_with(b"ax"), Some(1));

        assert_eq!(trie.matches_exactly(b"a"), Some(1));
        assert_eq!(trie.matches_exactly(b"ab"), Some(2));
        assert_eq!(trie.matches_exactly(b"abc"), Some(3));
        assert_eq!(trie.matches_exactly(b"abcd"), None);
        assert_eq!(trie.matches_exactly(b"ax"), None);
    }

    #[test]
    fn unmatched_input_keeps_last_value_for_starts_with() {
        let mut builder = TrieBuilder::new();
        builder.add(b"ab", 7);
        let trie = builder.build();

        assert_eq!(trie.starts_with(b"abX"), Some(7));
        assert_eq!(trie.matches_exactly(b"abX"), None);

        assert_eq!(trie.starts_with(b"a"), None);
        assert_eq!(trie.matches_exactly(b"a"), None);
        assert_eq!(trie.starts_with(b"X"), None);
        assert_eq!(trie.matches_exactly(b"X"), None);
    }

    #[test]
    fn empty_word_sets_root_value() {
        let mut builder = TrieBuilder::new();
        builder.add(b"", 99);
        builder.add(b"x", 1);
        let trie = builder.build();

        assert_eq!(trie.starts_with(b""), Some(99));
        assert_eq!(trie.matches_exactly(b""), Some(99));

        assert_eq!(trie.starts_with(b"x"), Some(1));
        assert_eq!(trie.matches_exactly(b"x"), Some(1));

        // Unmatched path still has the root value for starts_with.
        assert_eq!(trie.starts_with(b"z"), Some(99));
        assert_eq!(trie.matches_exactly(b"z"), None);
    }

    #[test]
    fn later_add_overwrites_same_word_value() {
        let mut builder = TrieBuilder::new();
        builder.add(b"key", 1);
        builder.add(b"key", 2);
        let trie = builder.build();

        assert_eq!(trie.starts_with(b"key"), Some(2));
        assert_eq!(trie.matches_exactly(b"key"), Some(2));
    }

    #[test]
    fn binary_magic_bytes() {
        let mut builder = TrieBuilder::new();
        builder.add(b"\x7fELF", 100);
        builder.add(b"PK\x03\x04", 200);
        builder.add(&[0xFF, 0xD8, 0xFF], 300);
        let trie = builder.build();

        assert_eq!(trie.starts_with(b"\x7fELF\x01\x01"), Some(100));
        assert_eq!(trie.matches_exactly(b"\x7fELF\x01\x01"), None);
        assert_eq!(trie.matches_exactly(b"\x7fELF"), Some(100));

        assert_eq!(trie.starts_with(b"PK\x03\x04extra"), Some(200));
        assert_eq!(trie.matches_exactly(b"PK\x03\x04extra"), None);
        assert_eq!(trie.matches_exactly(b"PK\x03\x04"), Some(200));

        assert_eq!(trie.starts_with(&[0xFF, 0xD8, 0xFF, 0xE0]), Some(300));
        assert_eq!(trie.matches_exactly(&[0xFF, 0xD8, 0xFF, 0xE0]), None);
        assert_eq!(trie.matches_exactly(&[0xFF, 0xD8, 0xFF]), Some(300));

        assert_eq!(trie.starts_with(b"\x7fEL"), None);
        assert_eq!(trie.matches_exactly(b"\x7fEL"), None);
    }

    #[test]
    fn many_children_branching() {
        // Force TrieChildren::Many (4+ distinct edges from one node).
        let mut builder = TrieBuilder::new();
        builder.add(b"a", 1);
        builder.add(b"b", 2);
        builder.add(b"c", 3);
        builder.add(b"d", 4);
        builder.add(b"e", 5);
        builder.add(b"m", 13);
        builder.add(b"z", 26);
        let trie = builder.build();

        assert_eq!(trie.starts_with(b"a"), Some(1));
        assert_eq!(trie.starts_with(b"c"), Some(3));
        assert_eq!(trie.starts_with(b"e"), Some(5));
        assert_eq!(trie.starts_with(b"m"), Some(13));
        assert_eq!(trie.starts_with(b"z"), Some(26));
        assert_eq!(trie.starts_with(b"f"), None);

        assert_eq!(trie.matches_exactly(b"a"), Some(1));
        assert_eq!(trie.matches_exactly(b"c"), Some(3));
        assert_eq!(trie.matches_exactly(b"e"), Some(5));
        assert_eq!(trie.matches_exactly(b"m"), Some(13));
        assert_eq!(trie.matches_exactly(b"z"), Some(26));
        assert_eq!(trie.matches_exactly(b"f"), None);
    }

    #[test]
    fn deep_chain() {
        let mut builder = TrieBuilder::new();
        builder.add(b"abcdefghijklmnopqrstuvwxyz", 1);
        let trie = builder.build();

        assert_eq!(trie.starts_with(b"abcdefghijklmnopqrstuvwxyz"), Some(1));
        assert_eq!(trie.matches_exactly(b"abcdefghijklmnopqrstuvwxyz"), Some(1));

        assert_eq!(trie.starts_with(b"abcdefghijklmnopqrstuvwxy"), None);
        assert_eq!(trie.matches_exactly(b"abcdefghijklmnopqrstuvwxy"), None);

        assert_eq!(trie.starts_with(b"abcdefghijklmnopqrstuvwxyz!"), Some(1));
        assert_eq!(trie.matches_exactly(b"abcdefghijklmnopqrstuvwxyz!"), None);
    }
}

mod one {
    use super::super::one::OneMatcher;
    use crate::ContentType;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestType {
        Pdf = 1,
        Zip = 2,
        Elf = 3,
    }

    impl ContentType for TestType {
        const COUNT: u16 = 4;

        fn as_u16(&self) -> u16 {
            *self as u16
        }

        fn from_u16(value: u16) -> Option<Self> {
            match value {
                1 => Some(Self::Pdf),
                2 => Some(Self::Zip),
                3 => Some(Self::Elf),
                _ => None,
            }
        }
    }

    #[test]
    fn exact_match() {
        let matcher = OneMatcher::new(TestType::Pdf, b"%PDF");

        assert_eq!(matcher.starts_with(b"%PDF"), Some(TestType::Pdf));
        assert_eq!(matcher.matches_exactly(b"%PDF"), Some(TestType::Pdf));
    }

    #[test]
    fn prefix_with_extra_bytes() {
        let matcher = OneMatcher::new(TestType::Pdf, b"%PDF");

        assert_eq!(matcher.starts_with(b"%PDF-1.7"), Some(TestType::Pdf));
        assert_eq!(matcher.matches_exactly(b"%PDF-1.7"), None);
    }

    #[test]
    fn shorter_or_different_input_returns_none() {
        let matcher = OneMatcher::new(TestType::Pdf, b"%PDF");

        assert_eq!(matcher.starts_with(b"%PD"), None);
        assert_eq!(matcher.matches_exactly(b"%PD"), None);
        assert_eq!(matcher.starts_with(b"%PDX"), None);
        assert_eq!(matcher.matches_exactly(b"%PDX"), None);
        assert_eq!(matcher.starts_with(b"PDF"), None);
        assert_eq!(matcher.matches_exactly(b"PDF"), None);
    }

    #[test]
    fn empty_input_with_non_empty_pattern_returns_none() {
        let matcher = OneMatcher::new(TestType::Zip, b"PK");

        assert_eq!(matcher.starts_with(b""), None);
        assert_eq!(matcher.matches_exactly(b""), None);
    }

    #[test]
    fn empty_pattern() {
        let matcher = OneMatcher::new(TestType::Pdf, b"");

        // starts_with: empty pattern is a prefix of every input.
        assert_eq!(matcher.starts_with(b""), Some(TestType::Pdf));
        assert_eq!(matcher.starts_with(b"anything"), Some(TestType::Pdf));
        // matches_exactly: only an empty input equals an empty pattern.
        assert_eq!(matcher.matches_exactly(b""), Some(TestType::Pdf));
        assert_eq!(matcher.matches_exactly(b"anything"), None);
    }

    #[test]
    fn single_byte_pattern() {
        let matcher = OneMatcher::new(TestType::Zip, b"P");

        assert_eq!(matcher.starts_with(b"P"), Some(TestType::Zip));
        assert_eq!(matcher.starts_with(b"PK"), Some(TestType::Zip));
        assert_eq!(matcher.starts_with(b"X"), None);
        assert_eq!(matcher.matches_exactly(b"P"), Some(TestType::Zip));
        assert_eq!(matcher.matches_exactly(b"PK"), None);
        assert_eq!(matcher.matches_exactly(b"X"), None);
    }

    #[test]
    fn binary_magic_bytes() {
        let elf = OneMatcher::new(TestType::Elf, b"\x7fELF");
        let zip = OneMatcher::new(TestType::Zip, b"PK\x03\x04");

        assert_eq!(elf.starts_with(b"\x7fELF"), Some(TestType::Elf));
        assert_eq!(elf.starts_with(b"\x7fELF\x01\x01"), Some(TestType::Elf));
        assert_eq!(elf.starts_with(b"\x7fEL"), None);
        assert_eq!(elf.matches_exactly(b"\x7fELF"), Some(TestType::Elf));
        assert_eq!(elf.matches_exactly(b"\x7fELF\x01\x01"), None);
        assert_eq!(elf.matches_exactly(b"\x7fEL"), None);

        assert_eq!(zip.starts_with(b"PK\x03\x04extra"), Some(TestType::Zip));
        assert_eq!(zip.matches_exactly(b"PK\x03\x04extra"), None);
        assert_eq!(zip.matches_exactly(b"PK\x03\x04"), Some(TestType::Zip));
        assert_eq!(zip.starts_with(b"PK\x03\x03"), None);
        assert_eq!(zip.matches_exactly(b"PK\x03\x03"), None);
    }

    #[test]
    fn returns_configured_content_type() {
        let pdf = OneMatcher::new(TestType::Pdf, b"magic");
        let zip = OneMatcher::new(TestType::Zip, b"magic");

        assert_eq!(pdf.starts_with(b"magic"), Some(TestType::Pdf));
        assert_eq!(zip.starts_with(b"magic"), Some(TestType::Zip));
        assert_eq!(pdf.matches_exactly(b"magic"), Some(TestType::Pdf));
        assert_eq!(zip.matches_exactly(b"magic"), Some(TestType::Zip));
    }
}

