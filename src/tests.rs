
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

mod plugin_list {
    use crate::plugin_list::PluginsList;
    use crate::ContentType;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestType {
        A = 0,
        B = 1,
        C = 2,
    }

    impl ContentType for TestType {
        const COUNT: u16 = 3;

        fn as_u16(&self) -> u16 {
            *self as u16
        }

        fn from_u16(value: u16) -> Option<Self> {
            match value {
                0 => Some(Self::A),
                1 => Some(Self::B),
                2 => Some(Self::C),
                _ => None,
            }
        }
    }

    fn typed(type_id: u16, priority: u8, value: &str) -> (u32, String) {
        ((type_id as u32) << 16 | priority as u32, value.to_string())
    }

    fn generic(priority: u8, value: &str) -> (u32, String) {
        (0xFFFF_0000 | priority as u32, value.to_string())
    }

    fn values_in_range(list: &mut PluginsList<String>, start: usize, end: usize) -> Vec<String> {
        (start..end)
            .map(|i| unsafe { list.get(i).clone() })
            .collect()
    }

    #[test]
    fn empty_with_types_has_no_ranges() {
        let list = PluginsList::<String>::new(vec![], TestType::COUNT);
        assert_eq!(list.len(), 0);
        assert_eq!(list.range(TestType::A), None);
        assert_eq!(list.range(TestType::B), None);
        assert_eq!(list.range(TestType::C), None);
        assert_eq!(list.generic_range(), None);
    }

    #[test]
    fn empty_with_zero_max_count() {
        let list = PluginsList::<String>::new(vec![], 0);
        assert_eq!(list.len(), 0);
        assert_eq!(list.range(TestType::A), None);
        assert_eq!(list.generic_range(), None);
    }

    #[test]
    fn single_typed_plugin() {
        let mut list = PluginsList::new(vec![typed(0, 1, "a")], TestType::COUNT);
        assert_eq!(list.len(), 1);
        assert_eq!(list.range(TestType::A), Some((0, 1)));
        assert_eq!(list.range(TestType::B), None);
        assert_eq!(list.generic_range(), None);
        assert_eq!(unsafe { list.get(0) }, "a");
    }

    #[test]
    fn multiple_plugins_same_type_sorted_by_priority() {
        // Insert out of priority order; lower priority value comes first.
        let mut list = PluginsList::new(
            vec![
                typed(0, 30, "p30"),
                typed(0, 10, "p10"),
                typed(0, 20, "p20"),
            ],
            TestType::COUNT,
        );
        assert_eq!(list.range(TestType::A), Some((0, 3)));
        assert_eq!(values_in_range(&mut list, 0, 3), ["p10", "p20", "p30"]);
        assert_eq!(list.generic_range(), None);
    }

    #[test]
    fn multiple_types_get_disjoint_ranges() {
        let mut list = PluginsList::new(
            vec![
                typed(2, 1, "c1"),
                typed(0, 1, "a1"),
                typed(1, 1, "b1"),
                typed(0, 2, "a2"),
                typed(2, 0, "c0"),
            ],
            TestType::COUNT,
        );
        assert_eq!(list.len(), 5);
        assert_eq!(list.range(TestType::A), Some((0, 2)));
        assert_eq!(list.range(TestType::B), Some((2, 3)));
        assert_eq!(list.range(TestType::C), Some((3, 5)));
        assert_eq!(values_in_range(&mut list, 0, 2), ["a1", "a2"]);
        assert_eq!(values_in_range(&mut list, 2, 3), ["b1"]);
        assert_eq!(values_in_range(&mut list, 3, 5), ["c0", "c1"]);
        assert_eq!(list.generic_range(), None);
    }

    #[test]
    fn missing_middle_type_returns_none() {
        let list = PluginsList::new(
            vec![typed(0, 0, "a"), typed(2, 0, "c")],
            TestType::COUNT,
        );
        assert_eq!(list.range(TestType::A), Some((0, 1)));
        assert_eq!(list.range(TestType::B), None);
        assert_eq!(list.range(TestType::C), Some((1, 2)));
    }

    #[test]
    fn only_last_type_populated() {
        let list = PluginsList::new(vec![typed(2, 5, "c")], TestType::COUNT);
        assert_eq!(list.range(TestType::A), None);
        assert_eq!(list.range(TestType::B), None);
        assert_eq!(list.range(TestType::C), Some((0, 1)));
        assert_eq!(list.generic_range(), None);
    }

    #[test]
    fn generics_only() {
        let mut list = PluginsList::new(
            vec![generic(2, "g2"), generic(0, "g0"), generic(1, "g1")],
            TestType::COUNT,
        );
        assert_eq!(list.range(TestType::A), None);
        assert_eq!(list.range(TestType::B), None);
        assert_eq!(list.range(TestType::C), None);
        assert_eq!(list.generic_range(), Some((0, 3)));
        assert_eq!(values_in_range(&mut list, 0, 3), ["g0", "g1", "g2"]);
    }

    #[test]
    fn generics_only_with_zero_max_count() {
        let mut list = PluginsList::new(vec![generic(1, "g")], 0);
        assert_eq!(list.len(), 1);
        assert_eq!(list.range(TestType::A), None);
        assert_eq!(list.generic_range(), Some((0, 1)));
        assert_eq!(unsafe { list.get(0) }, "g");
    }

    #[test]
    fn typed_then_generics() {
        let mut list = PluginsList::new(
            vec![
                generic(1, "g1"),
                typed(1, 0, "b"),
                typed(0, 0, "a0"),
                generic(0, "g0"),
                typed(0, 1, "a1"),
            ],
            TestType::COUNT,
        );
        assert_eq!(list.len(), 5);
        assert_eq!(list.range(TestType::A), Some((0, 2)));
        assert_eq!(list.range(TestType::B), Some((2, 3)));
        assert_eq!(list.range(TestType::C), None);
        assert_eq!(list.generic_range(), Some((3, 5)));
        assert_eq!(values_in_range(&mut list, 0, 2), ["a0", "a1"]);
        assert_eq!(values_in_range(&mut list, 2, 3), ["b"]);
        assert_eq!(values_in_range(&mut list, 3, 5), ["g0", "g1"]);
    }

    #[test]
    fn single_typed_then_single_generic() {
        let mut list = PluginsList::new(
            vec![typed(0, 0, "a"), generic(0, "g")],
            TestType::COUNT,
        );
        assert_eq!(list.range(TestType::A), Some((0, 1)));
        assert_eq!(list.generic_range(), Some((1, 2)));
        assert_eq!(unsafe { list.get(0) }, "a");
        assert_eq!(unsafe { list.get(1) }, "g");
    }

    #[test]
    fn range_for_type_beyond_max_count_returns_none() {
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        struct OutOfRange;
        impl ContentType for OutOfRange {
            const COUNT: u16 = 1;
            fn as_u16(&self) -> u16 {
                99
            }
            fn from_u16(_: u16) -> Option<Self> {
                None
            }
        }

        let list = PluginsList::new(vec![typed(0, 0, "a")], 1);
        assert_eq!(list.range(OutOfRange), None);
    }

    #[test]
    fn get_allows_mutation() {
        let mut list = PluginsList::new(vec![typed(0, 0, "old")], TestType::COUNT);
        unsafe {
            *list.get(0) = "new".to_string();
        }
        assert_eq!(unsafe { list.get(0) }, "new");
    }

    #[test]
    #[should_panic(expected = "Invalid type_id")]
    fn invalid_type_id_panics() {
        let _list = PluginsList::new(vec![typed(3, 0, "bad")], TestType::COUNT);
    }

    #[test]
    #[should_panic(expected = "Invalid type_id")]
    fn type_id_equal_to_max_count_panics() {
        // Valid typed ids are 0..max_count; max_count itself is invalid.
        let _list = PluginsList::new(vec![typed(TestType::COUNT, 0, "bad")], TestType::COUNT);
    }
}

