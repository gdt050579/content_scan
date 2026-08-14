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

mod packed_linear_list {
    use super::super::packed_linear_list::{Key, PackedLinearList};
    use crate::ContentType;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestType {
        A = 1,
        B = 2,
        C = 3,
        D = 4,
    }

    impl ContentType for TestType {
        const COUNT: u16 = 5;

        fn as_u16(&self) -> u16 {
            *self as u16
        }

        fn from_u16(value: u16) -> Option<Self> {
            match value {
                1 => Some(Self::A),
                2 => Some(Self::B),
                3 => Some(Self::C),
                4 => Some(Self::D),
                _ => None,
            }
        }
    }

    macro_rules! packed_key_tests {
        ($mod_name:ident, $key:ty, $oversize:expr) => {
            mod $mod_name {
                use super::*;

                #[test]
                fn empty_patterns_returns_none() {
                    assert!(PackedLinearList::<TestType, $key>::new(&[]).is_none());
                }

                #[test]
                fn more_than_sixteen_returns_none() {
                    let patterns: Vec<(TestType, &'static [u8])> = (0..17)
                        .map(|i| {
                            let pat: &'static [u8] = match i {
                                0 => b"0",
                                1 => b"1",
                                2 => b"2",
                                3 => b"3",
                                4 => b"4",
                                5 => b"5",
                                6 => b"6",
                                7 => b"7",
                                8 => b"8",
                                9 => b"9",
                                10 => b"a",
                                11 => b"b",
                                12 => b"c",
                                13 => b"d",
                                14 => b"e",
                                15 => b"f",
                                _ => b"g",
                            };
                            (TestType::A, pat)
                        })
                        .collect();
                    assert!(PackedLinearList::<TestType, $key>::new(&patterns).is_none());
                }

                #[test]
                fn empty_pattern_bytes_returns_none() {
                    assert!(
                        PackedLinearList::<TestType, $key>::new(&[(TestType::A, b"")]).is_none()
                    );
                }

                #[test]
                fn pattern_longer_than_key_width_returns_none() {
                    assert!(PackedLinearList::<TestType, $key>::new(&[(
                        TestType::A,
                        $oversize
                    )])
                    .is_none());
                }

                #[test]
                fn mixed_pattern_lengths_returns_none() {
                    assert!(PackedLinearList::<TestType, $key>::new(&[
                        (TestType::A, b"ab" as &[u8]),
                        (TestType::B, b"abc"),
                    ])
                    .is_none());
                }

                #[test]
                fn single_pattern_find() {
                    let list =
                        PackedLinearList::<TestType, $key>::new(&[(TestType::A, b"PK")]).unwrap();
                    assert_eq!(list.find(<$key>::pack(b"PK")), Some(TestType::A));
                    assert_eq!(list.find(<$key>::pack(b"P")), None);
                    assert_eq!(list.find(<$key>::pack(b"XX")), None);
                }

                #[test]
                fn multiple_patterns_find_each() {
                    let list = PackedLinearList::<TestType, $key>::new(&[
                        (TestType::A, b"%PDF"),
                        (TestType::B, b"PK\x03\x04"),
                        (TestType::C, b"\x7fELF"),
                    ])
                    .unwrap();

                    assert_eq!(list.find(<$key>::pack(b"%PDF")), Some(TestType::A));
                    assert_eq!(list.find(<$key>::pack(b"PK\x03\x04")), Some(TestType::B));
                    assert_eq!(list.find(<$key>::pack(b"\x7fELF")), Some(TestType::C));
                    assert_eq!(list.find(<$key>::pack(b"XXXX")), None);
                }

                #[test]
                fn sixteen_patterns_accepted() {
                    let patterns: [(TestType, &'static [u8]); 16] = [
                        (TestType::A, b"00"),
                        (TestType::B, b"01"),
                        (TestType::C, b"02"),
                        (TestType::D, b"03"),
                        (TestType::A, b"04"),
                        (TestType::B, b"05"),
                        (TestType::C, b"06"),
                        (TestType::D, b"07"),
                        (TestType::A, b"08"),
                        (TestType::B, b"09"),
                        (TestType::C, b"0a"),
                        (TestType::D, b"0b"),
                        (TestType::A, b"0c"),
                        (TestType::B, b"0d"),
                        (TestType::C, b"0e"),
                        (TestType::D, b"0f"),
                    ];
                    let list = PackedLinearList::<TestType, $key>::new(&patterns).unwrap();
                    assert_eq!(list.find(<$key>::pack(b"00")), Some(TestType::A));
                    assert_eq!(list.find(<$key>::pack(b"0f")), Some(TestType::D));
                    assert_eq!(list.find(<$key>::pack(b"10")), None);
                }

                #[test]
                fn duplicate_keys_returns_first() {
                    let list = PackedLinearList::<TestType, $key>::new(&[
                        (TestType::A, b"key"),
                        (TestType::B, b"key"),
                    ])
                    .unwrap();
                    assert_eq!(list.find(<$key>::pack(b"key")), Some(TestType::A));
                }

                #[test]
                fn single_byte_patterns() {
                    let list = PackedLinearList::<TestType, $key>::new(&[
                        (TestType::A, b"A"),
                        (TestType::B, b"B"),
                    ])
                    .unwrap();
                    assert_eq!(list.find(<$key>::pack(b"A")), Some(TestType::A));
                    assert_eq!(list.find(<$key>::pack(b"B")), Some(TestType::B));
                    assert_eq!(list.find(<$key>::pack(b"C")), None);
                }

                #[test]
                fn pack_zero_pads_shorter_than_width() {
                    let list =
                        PackedLinearList::<TestType, $key>::new(&[(TestType::A, b"AB")]).unwrap();
                    assert_eq!(list.find(<$key>::pack(b"AB")), Some(TestType::A));
                    if <$key>::WIDTH >= 4 {
                        assert_ne!(<$key>::pack(b"AB"), <$key>::pack(b"ABCD"));
                        assert_eq!(list.find(<$key>::pack(b"ABCD")), None);
                    }
                }
            }
        };
    }

    packed_key_tests!(u32_key, u32, b"12345" as &[u8]);
    packed_key_tests!(u64_key, u64, b"123456789" as &[u8]);

    mod u32_key_width {
        use super::*;

        #[test]
        fn accepts_full_width_pattern() {
            let list =
                PackedLinearList::<TestType, u32>::new(&[(TestType::A, b"abcd")]).unwrap();
            assert_eq!(list.find(u32::pack(b"abcd")), Some(TestType::A));
        }

        #[test]
        fn rejects_five_byte_pattern() {
            assert!(
                PackedLinearList::<TestType, u32>::new(&[(TestType::A, b"abcde")]).is_none()
            );
        }
    }

    mod u64_key_width {
        use super::*;

        #[test]
        fn accepts_full_width_pattern() {
            let list =
                PackedLinearList::<TestType, u64>::new(&[(TestType::A, b"abcdefgh")]).unwrap();
            assert_eq!(list.find(u64::pack(b"abcdefgh")), Some(TestType::A));
        }

        #[test]
        fn rejects_nine_byte_pattern() {
            assert!(
                PackedLinearList::<TestType, u64>::new(&[(TestType::A, b"abcdefghi")]).is_none()
            );
        }

        #[test]
        fn patterns_longer_than_u32_width() {
            let list = PackedLinearList::<TestType, u64>::new(&[
                (TestType::A, b"12345"),
                (TestType::B, b"67890"),
            ])
            .unwrap();
            assert_eq!(list.find(u64::pack(b"12345")), Some(TestType::A));
            assert_eq!(list.find(u64::pack(b"67890")), Some(TestType::B));
            assert_eq!(list.find(u64::pack(b"123456")), None);
        }
    }
}

mod fast_magic {
    use super::super::fast_magic::FastMagicMatcher;
    use crate::ContentType;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestType {
        Pk = 1,
        Gif = 2,
        Elf = 3,
        Pdf = 4,
        Riff = 5,
    }

    impl ContentType for TestType {
        const COUNT: u16 = 6;

        fn as_u16(&self) -> u16 {
            *self as u16
        }

        fn from_u16(value: u16) -> Option<Self> {
            match value {
                1 => Some(Self::Pk),
                2 => Some(Self::Gif),
                3 => Some(Self::Elf),
                4 => Some(Self::Pdf),
                5 => Some(Self::Riff),
                _ => None,
            }
        }
    }

    #[test]
    fn empty_patterns_returns_none() {
        assert!(FastMagicMatcher::<TestType>::new(&[]).is_none());
    }

    #[test]
    fn pattern_shorter_than_two_returns_none() {
        assert!(FastMagicMatcher::new(&[(TestType::Pk, b"P")]).is_none());
        assert!(FastMagicMatcher::new(&[(TestType::Pk, b"")]).is_none());
    }

    #[test]
    fn pattern_longer_than_four_returns_none() {
        assert!(FastMagicMatcher::new(&[(TestType::Pdf, b"%PDF-")]).is_none());
    }

    #[test]
    fn only_two_byte_patterns() {
        let matcher = FastMagicMatcher::new(&[
            (TestType::Pk, b"PK"),
            (TestType::Gif, b"BM"),
        ])
        .unwrap();

        assert_eq!(matcher.starts_with(b"PK"), Some(TestType::Pk));
        assert_eq!(matcher.starts_with(b"PKextra"), Some(TestType::Pk));
        assert_eq!(matcher.starts_with(b"BM"), Some(TestType::Gif));
        assert_eq!(matcher.starts_with(b"XX"), None);
        assert_eq!(matcher.starts_with(b"P"), None);

        assert_eq!(matcher.matches_exactly(b"PK"), Some(TestType::Pk));
        assert_eq!(matcher.matches_exactly(b"BM"), Some(TestType::Gif));
        assert_eq!(matcher.matches_exactly(b"PKextra"), None);
        assert_eq!(matcher.matches_exactly(b"P"), None);
        assert_eq!(matcher.matches_exactly(b"PK\x03\x04"), None);
    }

    #[test]
    fn only_three_byte_patterns() {
        let matcher = FastMagicMatcher::new(&[(TestType::Gif, b"GIF")]).unwrap();

        assert_eq!(matcher.starts_with(b"GIF"), Some(TestType::Gif));
        assert_eq!(matcher.starts_with(b"GIF89a"), Some(TestType::Gif));
        assert_eq!(matcher.starts_with(b"GI"), None);

        assert_eq!(matcher.matches_exactly(b"GIF"), Some(TestType::Gif));
        assert_eq!(matcher.matches_exactly(b"GIF89a"), None);
        assert_eq!(matcher.matches_exactly(b"GI"), None);
    }

    #[test]
    fn only_four_byte_patterns() {
        let matcher = FastMagicMatcher::new(&[
            (TestType::Elf, b"\x7fELF"),
            (TestType::Pdf, b"%PDF"),
            (TestType::Riff, b"RIFF"),
        ])
        .unwrap();

        assert_eq!(matcher.starts_with(b"\x7fELF"), Some(TestType::Elf));
        assert_eq!(matcher.starts_with(b"\x7fELF\x01"), Some(TestType::Elf));
        assert_eq!(matcher.starts_with(b"%PDF-1.7"), Some(TestType::Pdf));
        assert_eq!(matcher.starts_with(b"RIFF"), Some(TestType::Riff));
        assert_eq!(matcher.starts_with(b"\x7fEL"), None);

        assert_eq!(matcher.matches_exactly(b"\x7fELF"), Some(TestType::Elf));
        assert_eq!(matcher.matches_exactly(b"%PDF"), Some(TestType::Pdf));
        assert_eq!(matcher.matches_exactly(b"RIFF"), Some(TestType::Riff));
        assert_eq!(matcher.matches_exactly(b"\x7fELF\x01"), None);
        assert_eq!(matcher.matches_exactly(b"%PD"), None);
    }

    #[test]
    fn mixed_lengths_sorted_by_size() {
        // Must be sorted ascending by pattern length (API assumption).
        let matcher = FastMagicMatcher::new(&[
            (TestType::Pk, b"PK"),
            (TestType::Gif, b"GIF"),
            (TestType::Elf, b"\x7fELF"),
            (TestType::Pdf, b"%PDF"),
        ])
        .unwrap();

        assert_eq!(matcher.matches_exactly(b"PK"), Some(TestType::Pk));
        assert_eq!(matcher.matches_exactly(b"GIF"), Some(TestType::Gif));
        assert_eq!(matcher.matches_exactly(b"\x7fELF"), Some(TestType::Elf));
        assert_eq!(matcher.matches_exactly(b"%PDF"), Some(TestType::Pdf));
    }

    #[test]
    fn starts_with_prefers_longer_match() {
        // Length-4 is checked before length-2.
        let matcher = FastMagicMatcher::new(&[
            (TestType::Pk, b"PK"),
            (TestType::Riff, b"PK\x03\x04"),
        ])
        .unwrap();

        assert_eq!(matcher.starts_with(b"PK\x03\x04"), Some(TestType::Riff));
        assert_eq!(matcher.starts_with(b"PK\x03\x04extra"), Some(TestType::Riff));
        // Too short for the 4-byte pattern: fall through to length-2.
        assert_eq!(matcher.starts_with(b"PK"), Some(TestType::Pk));
        // 4-byte prefix does not match; fall through to length-2.
        assert_eq!(matcher.starts_with(b"PKxxxx"), Some(TestType::Pk));
        assert_eq!(matcher.matches_exactly(b"PK"), Some(TestType::Pk));
        assert_eq!(matcher.matches_exactly(b"PK\x03\x04"), Some(TestType::Riff));
    }

    #[test]
    fn starts_with_prefers_three_byte_over_two() {
        let matcher = FastMagicMatcher::new(&[
            (TestType::Pk, b"GI"),
            (TestType::Gif, b"GIF"),
        ])
        .unwrap();

        assert_eq!(matcher.starts_with(b"GIF89a"), Some(TestType::Gif));
        assert_eq!(matcher.starts_with(b"GI"), Some(TestType::Pk));
        assert_eq!(matcher.starts_with(b"GIxx"), Some(TestType::Pk));
    }

    #[test]
    fn starts_with_falls_through_to_longer_pattern() {
        let matcher = FastMagicMatcher::new(&[
            (TestType::Pk, b"BM"),
            (TestType::Elf, b"\x7fELF"),
        ])
        .unwrap();

        assert_eq!(matcher.starts_with(b"\x7fELF"), Some(TestType::Elf));
        assert_eq!(matcher.starts_with(b"BM"), Some(TestType::Pk));
        assert_eq!(matcher.starts_with(b"\x7fEL"), None);
    }

    #[test]
    fn skips_missing_middle_length() {
        let matcher = FastMagicMatcher::new(&[
            (TestType::Pk, b"PK"),
            (TestType::Pdf, b"%PDF"),
        ])
        .unwrap();

        assert_eq!(matcher.starts_with(b"PK"), Some(TestType::Pk));
        assert_eq!(matcher.starts_with(b"%PDF"), Some(TestType::Pdf));
        assert_eq!(matcher.matches_exactly(b"GIF"), None);
        assert_eq!(matcher.matches_exactly(b"XXX"), None);
    }

    #[test]
    fn short_input_cannot_match_longer_patterns() {
        let matcher = FastMagicMatcher::new(&[(TestType::Elf, b"\x7fELF")]).unwrap();

        assert_eq!(matcher.starts_with(b""), None);
        assert_eq!(matcher.starts_with(b"\x7f"), None);
        assert_eq!(matcher.starts_with(b"\x7fEL"), None);
        assert_eq!(matcher.matches_exactly(b""), None);
        assert_eq!(matcher.matches_exactly(b"\x7fEL"), None);
        assert_eq!(matcher.matches_exactly(b"\x7fELF\x00"), None);
    }

    #[test]
    fn duplicate_same_length_returns_first() {
        let matcher = FastMagicMatcher::new(&[
            (TestType::Pk, b"PK"),
            (TestType::Gif, b"PK"),
        ])
        .unwrap();

        assert_eq!(matcher.starts_with(b"PK"), Some(TestType::Pk));
        assert_eq!(matcher.matches_exactly(b"PK"), Some(TestType::Pk));
    }

    #[test]
    fn range_boundary_two_and_four() {
        let matcher = FastMagicMatcher::new(&[
            (TestType::Pk, b"ab"),
            (TestType::Pdf, b"abcd"),
        ])
        .unwrap();

        assert_eq!(matcher.matches_exactly(b"ab"), Some(TestType::Pk));
        assert_eq!(matcher.matches_exactly(b"abcd"), Some(TestType::Pdf));
        assert_eq!(matcher.starts_with(b"ab"), Some(TestType::Pk));
        assert_eq!(matcher.starts_with(b"abcdef"), Some(TestType::Pdf));
    }
}

