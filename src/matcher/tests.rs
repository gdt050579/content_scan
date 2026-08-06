
mod trie {
    use crate::trie::TrieBuilder;

    #[test]
    fn empty_trie_returns_none() {
        let trie = TrieBuilder::new().build();
        assert_eq!(trie.scan(b""), None);
        assert_eq!(trie.scan(b"abc"), None);
    }

    #[test]
    fn exact_match() {
        let mut builder = TrieBuilder::new();
        builder.add(b"hello", 42);
        let trie = builder.build();

        assert_eq!(trie.scan(b"hello"), Some(42));
        assert_eq!(trie.scan(b"hell"), None);
        assert_eq!(trie.scan(b"hello!"), Some(42));
        assert_eq!(trie.scan(b"world"), None);
    }

    #[test]
    fn multiple_distinct_words() {
        let mut builder = TrieBuilder::new();
        builder.add(b"cat", 1);
        builder.add(b"dog", 2);
        builder.add(b"bird", 3);
        let trie = builder.build();

        assert_eq!(trie.scan(b"cat"), Some(1));
        assert_eq!(trie.scan(b"dog"), Some(2));
        assert_eq!(trie.scan(b"bird"), Some(3));
        assert_eq!(trie.scan(b"cow"), None);
    }

    #[test]
    fn shared_prefix() {
        let mut builder = TrieBuilder::new();
        builder.add(b"app", 10);
        builder.add(b"apple", 20);
        builder.add(b"apply", 30);
        let trie = builder.build();

        assert_eq!(trie.scan(b"app"), Some(10));
        assert_eq!(trie.scan(b"apple"), Some(20));
        assert_eq!(trie.scan(b"apply"), Some(30));
        // Continues past "app" into unmatched bytes; last match wins.
        assert_eq!(trie.scan(b"application"), Some(10));
    }

    #[test]
    fn longest_matching_prefix_value() {
        let mut builder = TrieBuilder::new();
        builder.add(b"a", 1);
        builder.add(b"ab", 2);
        builder.add(b"abc", 3);
        let trie = builder.build();

        assert_eq!(trie.scan(b"a"), Some(1));
        assert_eq!(trie.scan(b"ab"), Some(2));
        assert_eq!(trie.scan(b"abc"), Some(3));
        assert_eq!(trie.scan(b"abcd"), Some(3));
        assert_eq!(trie.scan(b"ax"), Some(1));
    }

    #[test]
    fn unmatched_input_keeps_last_value() {
        let mut builder = TrieBuilder::new();
        builder.add(b"ab", 7);
        let trie = builder.build();

        assert_eq!(trie.scan(b"abX"), Some(7));
        assert_eq!(trie.scan(b"a"), None);
        assert_eq!(trie.scan(b"X"), None);
    }

    #[test]
    fn empty_word_sets_root_value() {
        let mut builder = TrieBuilder::new();
        builder.add(b"", 99);
        builder.add(b"x", 1);
        let trie = builder.build();

        assert_eq!(trie.scan(b""), Some(99));
        assert_eq!(trie.scan(b"x"), Some(1));
        // Unmatched path still has the root value.
        assert_eq!(trie.scan(b"z"), Some(99));
    }

    #[test]
    fn later_add_overwrites_same_word_value() {
        let mut builder = TrieBuilder::new();
        builder.add(b"key", 1);
        builder.add(b"key", 2);
        let trie = builder.build();

        assert_eq!(trie.scan(b"key"), Some(2));
    }

    #[test]
    fn binary_magic_bytes() {
        let mut builder = TrieBuilder::new();
        builder.add(b"\x7fELF", 100);
        builder.add(b"PK\x03\x04", 200);
        builder.add(&[0xFF, 0xD8, 0xFF], 300);
        let trie = builder.build();

        assert_eq!(trie.scan(b"\x7fELF\x01\x01"), Some(100));
        assert_eq!(trie.scan(b"PK\x03\x04extra"), Some(200));
        assert_eq!(trie.scan(&[0xFF, 0xD8, 0xFF, 0xE0]), Some(300));
        assert_eq!(trie.scan(b"\x7fEL"), None);
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

        assert_eq!(trie.scan(b"a"), Some(1));
        assert_eq!(trie.scan(b"c"), Some(3));
        assert_eq!(trie.scan(b"e"), Some(5));
        assert_eq!(trie.scan(b"m"), Some(13));
        assert_eq!(trie.scan(b"z"), Some(26));
        assert_eq!(trie.scan(b"f"), None);
    }

    #[test]
    fn deep_chain() {
        let mut builder = TrieBuilder::new();
        builder.add(b"abcdefghijklmnopqrstuvwxyz", 1);
        let trie = builder.build();

        assert_eq!(trie.scan(b"abcdefghijklmnopqrstuvwxyz"), Some(1));
        assert_eq!(trie.scan(b"abcdefghijklmnopqrstuvwxy"), None);
        assert_eq!(trie.scan(b"abcdefghijklmnopqrstuvwxyz!"), Some(1));
    }
}
