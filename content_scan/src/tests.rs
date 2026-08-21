mod plugin_list {
    use crate::analyzer_list::AnalyzerList;
    use crate::ContentType;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
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

    fn values_in_range(list: &mut AnalyzerList<String>, start: usize, end: usize) -> Vec<String> {
        (start..end).map(|i| unsafe { list.get(i).clone() }).collect()
    }

    #[test]
    fn empty_with_types_has_no_ranges() {
        let list = AnalyzerList::<String>::new(vec![], TestType::COUNT);
        assert_eq!(list.len(), 0);
        assert_eq!(list.range(TestType::A), None);
        assert_eq!(list.range(TestType::B), None);
        assert_eq!(list.range(TestType::C), None);
        assert_eq!(list.generic_range(), None);
    }

    #[test]
    fn empty_with_zero_max_count() {
        let list = AnalyzerList::<String>::new(vec![], 0);
        assert_eq!(list.len(), 0);
        assert_eq!(list.range(TestType::A), None);
        assert_eq!(list.generic_range(), None);
    }

    #[test]
    fn single_typed_plugin() {
        let mut list = AnalyzerList::new(vec![typed(0, 1, "a")], TestType::COUNT);
        assert_eq!(list.len(), 1);
        assert_eq!(list.range(TestType::A), Some((0, 1)));
        assert_eq!(list.range(TestType::B), None);
        assert_eq!(list.generic_range(), None);
        assert_eq!(unsafe { list.get(0) }, "a");
    }

    #[test]
    fn multiple_plugins_same_type_sorted_by_priority() {
        // Insert out of priority order; lower priority value comes first.
        let mut list = AnalyzerList::new(vec![typed(0, 30, "p30"), typed(0, 10, "p10"), typed(0, 20, "p20")], TestType::COUNT);
        assert_eq!(list.range(TestType::A), Some((0, 3)));
        assert_eq!(values_in_range(&mut list, 0, 3), ["p10", "p20", "p30"]);
        assert_eq!(list.generic_range(), None);
    }

    #[test]
    fn multiple_types_get_disjoint_ranges() {
        let mut list = AnalyzerList::new(
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
        let list = AnalyzerList::new(vec![typed(0, 0, "a"), typed(2, 0, "c")], TestType::COUNT);
        assert_eq!(list.range(TestType::A), Some((0, 1)));
        assert_eq!(list.range(TestType::B), None);
        assert_eq!(list.range(TestType::C), Some((1, 2)));
    }

    #[test]
    fn only_last_type_populated() {
        let list = AnalyzerList::new(vec![typed(2, 5, "c")], TestType::COUNT);
        assert_eq!(list.range(TestType::A), None);
        assert_eq!(list.range(TestType::B), None);
        assert_eq!(list.range(TestType::C), Some((0, 1)));
        assert_eq!(list.generic_range(), None);
    }

    #[test]
    fn generics_only() {
        let mut list = AnalyzerList::new(vec![generic(2, "g2"), generic(0, "g0"), generic(1, "g1")], TestType::COUNT);
        assert_eq!(list.range(TestType::A), None);
        assert_eq!(list.range(TestType::B), None);
        assert_eq!(list.range(TestType::C), None);
        assert_eq!(list.generic_range(), Some((0, 3)));
        assert_eq!(values_in_range(&mut list, 0, 3), ["g0", "g1", "g2"]);
    }

    #[test]
    fn generics_only_with_zero_max_count() {
        let mut list = AnalyzerList::new(vec![generic(1, "g")], 0);
        assert_eq!(list.len(), 1);
        assert_eq!(list.range(TestType::A), None);
        assert_eq!(list.generic_range(), Some((0, 1)));
        assert_eq!(unsafe { list.get(0) }, "g");
    }

    #[test]
    fn typed_then_generics() {
        let mut list = AnalyzerList::new(
            vec![generic(1, "g1"), typed(1, 0, "b"), typed(0, 0, "a0"), generic(0, "g0"), typed(0, 1, "a1")],
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
        let mut list = AnalyzerList::new(vec![typed(0, 0, "a"), generic(0, "g")], TestType::COUNT);
        assert_eq!(list.range(TestType::A), Some((0, 1)));
        assert_eq!(list.generic_range(), Some((1, 2)));
        assert_eq!(unsafe { list.get(0) }, "a");
        assert_eq!(unsafe { list.get(1) }, "g");
    }

    #[test]
    fn range_for_type_beyond_max_count_returns_none() {
        #[derive(Clone, Copy, PartialEq, Eq, Debug, Ord, PartialOrd)]
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

        let list = AnalyzerList::new(vec![typed(0, 0, "a")], 1);
        assert_eq!(list.range(OutOfRange), None);
    }

    #[test]
    fn get_allows_mutation() {
        let mut list = AnalyzerList::new(vec![typed(0, 0, "old")], TestType::COUNT);
        unsafe {
            *list.get(0) = "new".to_string();
        }
        assert_eq!(unsafe { list.get(0) }, "new");
    }

    #[test]
    #[should_panic(expected = "Invalid type_id")]
    fn invalid_type_id_panics() {
        let _list = AnalyzerList::new(vec![typed(3, 0, "bad")], TestType::COUNT);
    }

    #[test]
    #[should_panic(expected = "Invalid type_id")]
    fn type_id_equal_to_max_count_panics() {
        // Valid typed ids are 0..max_count; max_count itself is invalid.
        let _list = AnalyzerList::new(vec![typed(TestType::COUNT, 0, "bad")], TestType::COUNT);
    }
}

mod utils {
    use crate::utils::{get_extension, get_file_name};

    #[test]
    fn file_name_from_unix_path() {
        assert_eq!(get_file_name(b"/home/user/file.txt"), b"file.txt");
        assert_eq!(get_file_name(b"/file.txt"), b"file.txt");
    }

    #[test]
    fn file_name_from_windows_path() {
        assert_eq!(get_file_name(br"C:\Users\me\file.txt"), b"file.txt");
        assert_eq!(get_file_name(br"C:\file.txt"), b"file.txt");
    }

    #[test]
    fn file_name_mixed_separators_uses_last() {
        assert_eq!(get_file_name(br"C:\Users/me\docs/file.txt"), b"file.txt");
        assert_eq!(get_file_name(b"/home\\user/file.txt"), b"file.txt");
    }

    #[test]
    fn file_name_without_separator_returns_whole_path() {
        assert_eq!(get_file_name(b"file.txt"), b"file.txt");
        assert_eq!(get_file_name(b"README"), b"README");
    }

    #[test]
    fn file_name_trailing_separator_returns_empty() {
        assert_eq!(get_file_name(b"/home/user/"), b"");
        assert_eq!(get_file_name(br"C:\Users\"), b"");
    }

    #[test]
    fn file_name_empty_path() {
        assert_eq!(get_file_name(b""), b"");
    }

    #[test]
    fn extension_basic() {
        assert_eq!(get_extension(b"file.txt"), b"txt");
        assert_eq!(get_extension(b"archive.tar.gz"), b"gz");
    }

    #[test]
    fn extension_no_dot_returns_empty() {
        assert_eq!(get_extension(b"README"), b"");
        assert_eq!(get_extension(b"Makefile"), b"");
    }

    #[test]
    fn extension_leading_dot_file() {
        // Dotfile with no further extension: everything after the only dot.
        assert_eq!(get_extension(b".gitignore"), b"gitignore");
        assert_eq!(get_extension(b".tar.gz"), b"gz");
    }

    #[test]
    fn extension_trailing_dot_returns_empty() {
        assert_eq!(get_extension(b"file."), b"");
    }

    #[test]
    fn extension_empty_name() {
        assert_eq!(get_extension(b""), b"");
        assert_eq!(get_extension(b"."), b"");
    }

    #[test]
    fn file_name_then_extension() {
        let name = get_file_name(br"C:\docs\report.PDF");
        assert_eq!(name, b"report.PDF");
        assert_eq!(get_extension(name), b"PDF");
    }
}

mod content_path {
    use crate::ContentPath;
    use std::path::{Path, PathBuf};

    #[test]
    fn from_str_and_owned_string_are_lossless() {
        let from_ref: ContentPath = "virtual://a".into();
        let owned = String::from("virtual://b");
        let from_owned: ContentPath = owned.clone().into();
        let from_string_ref: ContentPath = (&owned).into();
        assert!(from_ref.is_lossless());
        assert_eq!(from_ref.as_printable_string(), "virtual://a");
        assert!(from_owned.is_lossless());
        assert_eq!(from_owned.as_printable_string(), "virtual://b");
        assert_eq!(from_string_ref.as_printable_string(), "virtual://b");
    }

    #[test]
    fn from_path_and_pathbuf_use_from_os() {
        let p = Path::new("C:\\docs\\file.txt");
        let from_ref: ContentPath = p.into();
        let buf = PathBuf::from("C:\\docs\\file.txt");
        let from_owned: ContentPath = buf.clone().into();
        let from_buf_ref: ContentPath = (&buf).into();
        assert!(from_ref.is_lossless());
        assert_eq!(from_ref.as_path(), p);
        assert_eq!(from_owned.as_path(), p);
        assert_eq!(from_buf_ref.as_printable_string(), "C:\\docs\\file.txt");
    }
}

mod filter {
    use crate::{ContentPath, Filter, FilterBuilder, Precedence};

    fn process(filter: &mut Filter, path: &str) -> bool {
        filter.should_process(&ContentPath::from_str(path), 1)
    }

    #[test]
    fn higher_precedence_exclude_overrides_earlier_include() {
        let mut filter = FilterBuilder::new()
            .include_extensions(Precedence::Low, &["txt"])
            .exclude_extensions(Precedence::Highest, &["txt"])
            .allow_the_rest()
            .build();
        assert!(!process(&mut filter, "notes.txt"));
        assert!(process(&mut filter, "notes.rs"));
    }

    #[test]
    fn higher_precedence_include_overrides_earlier_exclude() {
        let mut filter = FilterBuilder::new()
            .exclude_extensions(Precedence::Low, &["txt"])
            .include_extensions(Precedence::High, &["txt"])
            .deny_the_rest()
            .build();
        assert!(process(&mut filter, "notes.txt"));
        assert!(!process(&mut filter, "notes.rs"));
    }

    #[test]
    fn same_precedence_keeps_insertion_order() {
        let mut include_first = FilterBuilder::new()
            .include_extensions(Precedence::Medium, &["txt"])
            .exclude_extensions(Precedence::Medium, &["txt"])
            .deny_the_rest()
            .build();
        assert!(process(&mut include_first, "notes.txt"));

        let mut exclude_first = FilterBuilder::new()
            .exclude_extensions(Precedence::Medium, &["txt"])
            .include_extensions(Precedence::Medium, &["txt"])
            .allow_the_rest()
            .build();
        assert!(!process(&mut exclude_first, "notes.txt"));
    }

    #[test]
    fn include_extensions_match_regardless_of_path_case() {
        let mut filter = FilterBuilder::new()
            .include_extensions(Precedence::Medium, &["jpg"])
            .deny_the_rest()
            .build();
        assert!(process(&mut filter, "Photo.JPG"));
        assert!(process(&mut filter, "photo.jpg"));
        assert!(process(&mut filter, "photo.Jpg"));
        assert!(process(&mut filter, r"C:\Photos\IMG.JPG"));
        assert!(process(&mut filter, "/home/me/pic.JpG"));
        assert!(!process(&mut filter, "notes.txt"));
    }

    #[test]
    fn include_extensions_lowercase_registered_patterns() {
        let mut filter = FilterBuilder::new()
            .include_extensions(Precedence::Medium, &["JPG", "Bmp"])
            .deny_the_rest()
            .build();
        assert!(process(&mut filter, "photo.jpg"));
        assert!(process(&mut filter, "photo.JPG"));
        assert!(process(&mut filter, "x.bmp"));
        assert!(process(&mut filter, "x.BMP"));
        assert!(!process(&mut filter, "x.png"));
    }

    #[test]
    fn exclude_extensions_are_ascii_case_insensitive() {
        let mut filter = FilterBuilder::new()
            .exclude_extensions(Precedence::High, &["tmp"])
            .allow_the_rest()
            .build();
        assert!(!process(&mut filter, "scratch.TMP"));
        assert!(!process(&mut filter, "scratch.tmp"));
        assert!(!process(&mut filter, "scratch.Tmp"));
        assert!(process(&mut filter, "notes.txt"));
    }

    #[test]
    fn include_file_names_are_ascii_case_insensitive() {
        let mut filter = FilterBuilder::new()
            .include_file_names(Precedence::Medium, &["Makefile"])
            .deny_the_rest()
            .build();
        assert!(process(&mut filter, "Makefile"));
        assert!(process(&mut filter, "makefile"));
        assert!(process(&mut filter, "MAKEFILE"));
        assert!(process(&mut filter, "/src/Makefile"));
        assert!(process(&mut filter, r"C:\src\makefile"));
        assert!(!process(&mut filter, "makefile.txt"));
        assert!(!process(&mut filter, "notes.rs"));
    }

    #[test]
    fn exclude_file_names_are_ascii_case_insensitive() {
        let mut filter = FilterBuilder::new()
            .exclude_file_names(Precedence::High, &["Cargo.lock"])
            .allow_the_rest()
            .build();
        assert!(!process(&mut filter, "Cargo.lock"));
        assert!(!process(&mut filter, "cargo.lock"));
        assert!(!process(&mut filter, "CARGO.LOCK"));
        assert!(!process(&mut filter, r"C:\proj\Cargo.lock"));
        assert!(process(&mut filter, "Cargo.toml"));
    }

    #[test]
    fn last_extension_component_is_matched_case_insensitively() {
        let mut filter = FilterBuilder::new()
            .include_extensions(Precedence::Medium, &["gz"])
            .deny_the_rest()
            .build();
        assert!(process(&mut filter, "archive.tar.gz"));
        assert!(process(&mut filter, "archive.TAR.GZ"));
        assert!(process(&mut filter, "archive.Tar.Gz"));
        assert!(!process(&mut filter, "archive.tar"));
    }
}

mod identify {
    use crate::*;

    #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ContentType)]
    #[repr(u16)]
    enum Ty {
        Tagged,
        Custom,
        Fallback,
        ByExt,
        ByName,
    }

    struct TaggedId;
    impl ContentIdentifier<Ty> for TaggedId {
        fn identify_method(&self) -> Option<IdentifyMethod> {
            Some(IdentifyMethod::Magic(b"TAG"))
        }
        fn validate(&self, _: &mut dyn Content<Ty>) -> bool {
            true
        }
    }

    struct TaggedButRejectsId;
    impl ContentIdentifier<Ty> for TaggedButRejectsId {
        fn identify_method(&self) -> Option<IdentifyMethod> {
            Some(IdentifyMethod::Magic(b"TAG"))
        }
        fn validate(&self, _: &mut dyn Content<Ty>) -> bool {
            false
        }
    }

    /// Example: no `IdentifyMethod` — decide from the path in `validate`.
    struct CustomPathId;
    impl ContentIdentifier<Ty> for CustomPathId {
        fn identify_method(&self) -> Option<IdentifyMethod> {
            None
        }
        fn validate(&self, content: &mut dyn Content<Ty>) -> bool {
            content.path().as_printable_string().ends_with(".custom")
        }
    }

    /// Example: catch-all custom identifier (any non-empty payload).
    struct NonEmptyId;
    impl ContentIdentifier<Ty> for NonEmptyId {
        fn identify_method(&self) -> Option<IdentifyMethod> {
            None
        }
        fn validate(&self, content: &mut dyn Content<Ty>) -> bool {
            content.size() > 0
        }
    }

    /// Custom identifier that inspects payload bytes past the 16-byte magic window.
    struct PastMagicWindowId;
    impl ContentIdentifier<Ty> for PastMagicWindowId {
        fn identify_method(&self) -> Option<IdentifyMethod> {
            None
        }
        fn validate(&self, content: &mut dyn Content<Ty>) -> bool {
            matches!(content.read(16, 4), Some(b) if b == b"MARK")
        }
    }

    struct ExtId;
    impl ContentIdentifier<Ty> for ExtId {
        fn identify_method(&self) -> Option<IdentifyMethod> {
            Some(IdentifyMethod::Extension("txt"))
        }
        fn validate(&self, _: &mut dyn Content<Ty>) -> bool {
            true
        }
    }

    struct MixedCaseExtId;
    impl ContentIdentifier<Ty> for MixedCaseExtId {
        fn identify_method(&self) -> Option<IdentifyMethod> {
            Some(IdentifyMethod::Extension("TXT"))
        }
        fn validate(&self, _: &mut dyn Content<Ty>) -> bool {
            true
        }
    }

    struct ExtsId;
    impl ContentIdentifier<Ty> for ExtsId {
        fn identify_method(&self) -> Option<IdentifyMethod> {
            Some(IdentifyMethod::Extensions(&["jpg", "Jpeg"]))
        }
        fn validate(&self, _: &mut dyn Content<Ty>) -> bool {
            true
        }
    }

    struct NameId;
    impl ContentIdentifier<Ty> for NameId {
        fn identify_method(&self) -> Option<IdentifyMethod> {
            Some(IdentifyMethod::Name("Makefile"))
        }
        fn validate(&self, _: &mut dyn Content<Ty>) -> bool {
            true
        }
    }

    struct NamesId;
    impl ContentIdentifier<Ty> for NamesId {
        fn identify_method(&self) -> Option<IdentifyMethod> {
            Some(IdentifyMethod::Names(&["Cargo.lock"]))
        }
        fn validate(&self, _: &mut dyn Content<Ty>) -> bool {
            true
        }
    }

    struct Exact16MagicId;
    impl ContentIdentifier<Ty> for Exact16MagicId {
        fn identify_method(&self) -> Option<IdentifyMethod> {
            Some(IdentifyMethod::Magic(b"0123456789ABCDEF"))
        }
        fn validate(&self, _: &mut dyn Content<Ty>) -> bool {
            true
        }
    }

    struct TooLongMagicId;
    impl ContentIdentifier<Ty> for TooLongMagicId {
        fn identify_method(&self) -> Option<IdentifyMethod> {
            Some(IdentifyMethod::Magic(b"0123456789ABCDEF!"))
        }
        fn validate(&self, _: &mut dyn Content<Ty>) -> bool {
            true
        }
    }

    struct TooLongMultipleMagicId;
    impl ContentIdentifier<Ty> for TooLongMultipleMagicId {
        fn identify_method(&self) -> Option<IdentifyMethod> {
            Some(IdentifyMethod::MultipleMagic(&[b"OK", b"0123456789ABCDEF!"]))
        }
        fn validate(&self, _: &mut dyn Content<Ty>) -> bool {
            true
        }
    }

    fn identified_type(scanner: &mut Scanner<Ty>, buf: &[u8], path: &str) -> Option<Ty> {
        let mut content = BufferContent::<Ty>::new(buf, path);
        let res = scanner.scan(&mut content, true);
        res.root().and_then(|h| res.content_type(h))
    }

    #[test]
    fn custom_identifier_classifies_when_identify_method_is_none() {
        let mut scanner = ScannerBuilder::new().add_identifier(Ty::Custom, CustomPathId).build();
        assert_eq!(identified_type(&mut scanner, b"hello", "blob.custom"), Some(Ty::Custom));
        assert_eq!(identified_type(&mut scanner, b"hello", "blob.bin"), None);
    }

    #[test]
    fn custom_identifier_runs_after_magic_validate_rejects() {
        let mut scanner = ScannerBuilder::new()
            .add_identifier(Ty::Tagged, TaggedButRejectsId)
            .add_identifier(Ty::Custom, CustomPathId)
            .build();
        assert_eq!(identified_type(&mut scanner, b"TAG payload", "blob.custom"), Some(Ty::Custom));
    }

    #[test]
    fn magic_match_still_wins_over_custom_identifier() {
        let mut scanner = ScannerBuilder::new()
            .add_identifier(Ty::Custom, CustomPathId)
            .add_identifier(Ty::Tagged, TaggedId)
            .build();
        assert_eq!(identified_type(&mut scanner, b"TAG payload", "blob.custom"), Some(Ty::Tagged));
    }

    #[test]
    fn custom_identifiers_are_tried_in_registration_order() {
        let mut scanner = ScannerBuilder::new()
            .add_identifier(Ty::Custom, CustomPathId)
            .add_identifier(Ty::Fallback, NonEmptyId)
            .build();
        assert_eq!(identified_type(&mut scanner, b"hello", "blob.custom"), Some(Ty::Custom));
        assert_eq!(identified_type(&mut scanner, b"hello", "blob.bin"), Some(Ty::Fallback));
        assert_eq!(identified_type(&mut scanner, b"", "empty.bin"), None);
    }

    #[test]
    fn custom_identifier_can_read_payload_bytes() {
        let mut scanner = ScannerBuilder::new().add_identifier(Ty::Custom, PastMagicWindowId).build();
        let mut matching = [0u8; 20];
        matching[16..20].copy_from_slice(b"MARK");
        assert_eq!(identified_type(&mut scanner, &matching, "blob.bin"), Some(Ty::Custom));
        assert_eq!(identified_type(&mut scanner, &[0u8; 20], "blob.bin"), None);
        assert_eq!(identified_type(&mut scanner, b"short", "blob.bin"), None);
    }

    #[test]
    fn extension_match_is_ascii_case_insensitive() {
        let mut scanner = ScannerBuilder::new().add_identifier(Ty::ByExt, ExtId).build();
        assert_eq!(identified_type(&mut scanner, b"hello", "notes.txt"), Some(Ty::ByExt));
        assert_eq!(identified_type(&mut scanner, b"hello", "Notes.TXT"), Some(Ty::ByExt));
        assert_eq!(identified_type(&mut scanner, b"hello", "notes.Txt"), Some(Ty::ByExt));
        assert_eq!(identified_type(&mut scanner, b"hello", r"C:\docs\report.TXT"), Some(Ty::ByExt));
        assert_eq!(identified_type(&mut scanner, b"hello", "archive.tar.txt"), Some(Ty::ByExt));
        assert_eq!(identified_type(&mut scanner, b"hello", "notes.rs"), None);
    }

    #[test]
    fn mixed_case_registered_extension_still_matches() {
        let mut scanner = ScannerBuilder::new().add_identifier(Ty::ByExt, MixedCaseExtId).build();
        assert_eq!(identified_type(&mut scanner, b"hello", "notes.txt"), Some(Ty::ByExt));
        assert_eq!(identified_type(&mut scanner, b"hello", "NOTES.TXT"), Some(Ty::ByExt));
    }

    #[test]
    fn extensions_list_is_ascii_case_insensitive() {
        let mut scanner = ScannerBuilder::new().add_identifier(Ty::ByExt, ExtsId).build();
        assert_eq!(identified_type(&mut scanner, b"hello", "photo.jpg"), Some(Ty::ByExt));
        assert_eq!(identified_type(&mut scanner, b"hello", "Photo.JPG"), Some(Ty::ByExt));
        assert_eq!(identified_type(&mut scanner, b"hello", "shot.JPEG"), Some(Ty::ByExt));
        assert_eq!(identified_type(&mut scanner, b"hello", "shot.jpeg"), Some(Ty::ByExt));
        assert_eq!(identified_type(&mut scanner, b"hello", "shot.png"), None);
    }

    #[test]
    fn name_match_is_ascii_case_insensitive() {
        let mut scanner = ScannerBuilder::new().add_identifier(Ty::ByName, NameId).build();
        assert_eq!(identified_type(&mut scanner, b"hello", "Makefile"), Some(Ty::ByName));
        assert_eq!(identified_type(&mut scanner, b"hello", "makefile"), Some(Ty::ByName));
        assert_eq!(identified_type(&mut scanner, b"hello", "MAKEFILE"), Some(Ty::ByName));
        assert_eq!(identified_type(&mut scanner, b"hello", "/src/Makefile"), Some(Ty::ByName));
        assert_eq!(identified_type(&mut scanner, b"hello", r"C:\src\makefile"), Some(Ty::ByName));
        assert_eq!(identified_type(&mut scanner, b"hello", "makefile.txt"), None);
        assert_eq!(identified_type(&mut scanner, b"hello", "notes.rs"), None);
    }

    #[test]
    fn names_list_is_ascii_case_insensitive() {
        let mut scanner = ScannerBuilder::new().add_identifier(Ty::ByName, NamesId).build();
        assert_eq!(identified_type(&mut scanner, b"hello", "Cargo.lock"), Some(Ty::ByName));
        assert_eq!(identified_type(&mut scanner, b"hello", "cargo.lock"), Some(Ty::ByName));
        assert_eq!(identified_type(&mut scanner, b"hello", "CARGO.LOCK"), Some(Ty::ByName));
        assert_eq!(identified_type(&mut scanner, b"hello", r"C:\proj\Cargo.lock"), Some(Ty::ByName));
        assert_eq!(identified_type(&mut scanner, b"hello", "Cargo.toml"), None);
    }

    #[test]
    fn magic_of_exactly_16_bytes_still_matches() {
        let mut scanner = ScannerBuilder::new().add_identifier(Ty::Tagged, Exact16MagicId).build();
        assert_eq!(identified_type(&mut scanner, b"0123456789ABCDEF rest", "blob.bin"), Some(Ty::Tagged));
        assert_eq!(identified_type(&mut scanner, b"0123456789ABCDE?", "blob.bin"), None);
    }

    #[test]
    #[should_panic(expected = "at most 16 bytes")]
    fn magic_longer_than_16_bytes_panics_at_build() {
        ScannerBuilder::new().add_identifier(Ty::Tagged, TooLongMagicId).build();
    }

    #[test]
    #[should_panic(expected = "at most 16 bytes")]
    fn multiple_magic_item_longer_than_16_bytes_panics_at_build() {
        ScannerBuilder::new().add_identifier(Ty::Tagged, TooLongMultipleMagicId).build();
    }
}

mod max_depth {
    use crate::*;

    #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ContentType)]
    #[repr(u16)]
    enum Ty {
        Nest,
    }

    /// Emits one smaller child so scans form a single chain of nested objects.
    struct NestExtractor;

    struct NestSession {
        content: OwnedContentPtr<Ty>,
        entry: Entry,
        done: bool,
    }

    impl ContentExtractor<Ty> for NestExtractor {
        fn create_session(&mut self, content: OwnedContentPtr<Ty>, _: &ExtractionContext) -> Option<Box<dyn ExtractionSession<Ty>>> {
            if content.size() == 0 {
                return None;
            }
            Some(Box::new(NestSession {
                content,
                entry: Entry::default(),
                done: false,
            }))
        }
    }

    impl ExtractionSession<Ty> for NestSession {
        fn advance(&mut self) -> Option<&Entry> {
            if self.done {
                return None;
            }
            self.done = true;
            self.entry.path.set_from_str("child");
            self.entry.size = self.content.size().saturating_sub(1);
            self.entry.skip_from_filtering = false;
            Some(&self.entry)
        }
        fn extract(&mut self) -> Option<Box<dyn Content<Ty>>> {
            let n = self.content.size().saturating_sub(1) as usize;
            Some(Box::new(BufferContent::<Ty>::with_content_type(&vec![0u8; n], "child", Ty::Nest)))
        }
    }

    fn scanned(max_depth: u32) -> u32 {
        let mut scanner = ScannerBuilder::new().max_depth(max_depth).add_extractor(Ty::Nest, NestExtractor).build();
        let mut content = BufferContent::<Ty>::with_content_type(&[0u8; 16], "root", Ty::Nest);
        scanner.scan(&mut content, true).objects_scanned()
    }

    #[test]
    fn max_depth_one_scans_only_the_root() {
        assert_eq!(scanned(1), 1);
    }

    #[test]
    fn max_depth_three_scans_three_objects() {
        assert_eq!(scanned(3), 3);
    }

    #[test]
    fn max_depth_does_not_scan_an_extra_level() {
        assert_eq!(scanned(8), 8);
    }
}

mod local_varmap {
    use crate::*;

    #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ContentType)]
    #[repr(u16)]
    enum Ty {
        Leaf,
        Nest,
    }

    #[derive(Dependencies)]
    #[Dependencies(name = "TagAnalyzer", requires = "Leaf")]
    struct TagAnalyzer(u32);
    impl ContentAnalyzer<Ty> for TagAnalyzer {
        fn analyze(&mut self, _: &mut dyn Content<Ty>, context: &mut Context<Ty>) -> NextAction {
            context.local().set(var!("tag"), self.0);
            NextAction::Continue
        }
    }

    #[derive(Dependencies)]
    #[Dependencies(name = "SkipAfterLocal")]
    struct SkipAfterLocal;
    impl ContentAnalyzer<Ty> for SkipAfterLocal {
        fn analyze(&mut self, _: &mut dyn Content<Ty>, context: &mut Context<Ty>) -> NextAction {
            context.local().set(var!("tag"), 99u32);
            NextAction::Skip
        }
    }

    struct OneChildExtractor;

    struct OneChildSession {
        entry: Entry,
        done: bool,
    }

    impl ContentExtractor<Ty> for OneChildExtractor {
        fn create_session(&mut self, _: OwnedContentPtr<Ty>, _: &ExtractionContext) -> Option<Box<dyn ExtractionSession<Ty>>> {
            Some(Box::new(OneChildSession {
                entry: Entry::default(),
                done: false,
            }))
        }
    }

    impl ExtractionSession<Ty> for OneChildSession {
        fn advance(&mut self) -> Option<&Entry> {
            if self.done {
                return None;
            }
            self.done = true;
            self.entry.path.set_from_str("child");
            self.entry.size = 1;
            self.entry.skip_from_filtering = false;
            Some(&self.entry)
        }
        fn extract(&mut self) -> Option<Box<dyn Content<Ty>>> {
            Some(Box::new(BufferContent::<Ty>::with_content_type(b"x", "child", Ty::Leaf)))
        }
    }

    fn tag(res: &ScanResult<Ty>, handle: ScanContentHandle) -> Option<u32> {
        res.local(handle).and_then(|vm| vm.get::<u32>(var!("tag")))
    }

    #[test]
    fn local_map_is_visible_on_the_result_tree() {
        let mut scanner = ScannerBuilder::new().add_analyzer(Ty::Leaf, 0, TagAnalyzer(1)).build();
        let mut content = BufferContent::<Ty>::with_content_type(b"x", "root", Ty::Leaf);
        let res = scanner.scan(&mut content, true);
        let root = res.root().unwrap();
        assert_eq!(tag(&res, root), Some(1));
    }

    #[test]
    fn object_that_never_calls_local_has_no_map() {
        let mut scanner = ScannerBuilder::<Ty>::new().build();
        let mut content = BufferContent::<Ty>::with_content_type(b"x", "root", Ty::Leaf);
        let res = scanner.scan(&mut content, true);
        let root = res.root().unwrap();
        assert!(res.local(root).is_none());
    }

    #[test]
    fn parent_and_child_keep_distinct_maps() {
        let mut scanner = ScannerBuilder::new()
            .add_analyzer(Ty::Nest, 0, TagAnalyzer(1))
            .add_analyzer(Ty::Leaf, 0, TagAnalyzer(2))
            .add_extractor(Ty::Nest, OneChildExtractor)
            .build();
        let mut content = BufferContent::<Ty>::with_content_type(b"x", "root", Ty::Nest);
        let res = scanner.scan(&mut content, true);
        let root = res.root().unwrap();
        let child = res.child(root).unwrap();
        assert_eq!(tag(&res, root), Some(1));
        assert_eq!(tag(&res, child), Some(2));
        assert!(!std::ptr::eq(res.local(root).unwrap(), res.local(child).unwrap()));
    }

    #[test]
    fn skip_after_local_still_keeps_the_map() {
        let mut scanner = ScannerBuilder::new().add_analyzer(Ty::Leaf, 0, SkipAfterLocal).build();
        let mut content = BufferContent::<Ty>::with_content_type(b"x", "root", Ty::Leaf);
        let res = scanner.scan(&mut content, true);
        let root = res.root().unwrap();
        assert_eq!(tag(&res, root), Some(99));
    }
}

mod request_extract {
    use crate::*;

    #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ContentType)]
    #[repr(u16)]
    enum Ty {
        Root,
        Slice,
        SliceKid,
        Mid,
        MidKid,
        Inner,
        InnerKid,
        Left,
        LeftKid,
        Right,
        RightKid,
        Deep,
        DeepKid,
        Missing,
    }

    #[derive(Dependencies)]
    #[Dependencies(name = "Request")]
    struct Request(Ty);
    impl ContentAnalyzer<Ty> for Request {
        fn analyze(&mut self, _: &mut dyn Content<Ty>, context: &mut Context<Ty>) -> NextAction {
            context.request_extract(self.0).emit();
            NextAction::Continue
        }
    }


    #[derive(Dependencies)]
    #[Dependencies(name = "RequestTwo", requires = "Request")]
    struct RequestTwo(Ty, Ty);
    impl ContentAnalyzer<Ty> for RequestTwo {
        fn analyze(&mut self, _: &mut dyn Content<Ty>, context: &mut Context<Ty>) -> NextAction {
            context.request_extract(self.0).emit();
            context.request_extract(self.1).emit();
            NextAction::Continue
        }
    }


    #[derive(Dependencies)]
    #[Dependencies(name = "RequestSlice")]
    struct RequestSlice;
    impl ContentAnalyzer<Ty> for RequestSlice {
        fn analyze(&mut self, _: &mut dyn Content<Ty>, context: &mut Context<Ty>) -> NextAction {
            context.request_extract(Ty::Slice).at(2).len(3).param(var!("tag"), 9u32).emit();
            NextAction::Continue
        }
    }

    #[derive(Dependencies)]
    #[Dependencies(name = "DropWithoutEmit")]
    struct DropWithoutEmit;
    impl ContentAnalyzer<Ty> for DropWithoutEmit {
        fn analyze(&mut self, _: &mut dyn Content<Ty>, context: &mut Context<Ty>) -> NextAction {
            let _ = context.request_extract(Ty::Slice).at(0).param(var!("tag"), 1u32);
            NextAction::Continue
        }
    }

    #[derive(Dependencies)]
    #[Dependencies(name = "RequestMissing")]
    struct RequestMissing;
    impl ContentAnalyzer<Ty> for RequestMissing {
        fn analyze(&mut self, _: &mut dyn Content<Ty>, context: &mut Context<Ty>) -> NextAction {
            context.request_extract(Ty::Missing).param(var!("tag"), 1u32).emit();
            NextAction::Continue
        }
    }

    #[derive(Dependencies)]
    #[Dependencies(name = "Tag")]
    struct Tag(u32);
    impl ContentAnalyzer<Ty> for Tag {
        fn analyze(&mut self, content: &mut dyn Content<Ty>, context: &mut Context<Ty>) -> NextAction {
            context.local().set(var!("tag"), self.0);
            if let Some(b) = content.read(0, 1).and_then(|s| s.first().copied()) {
                context.local().set(var!("first"), b as u32);
            }
            context.local().set(var!("size"), content.size() as u32);
            NextAction::Continue
        }
    }

    struct EmitOnce {
        child_type: Ty,
        child_path: &'static str,
    }
    impl EmitOnce {
        fn new(child_type: Ty, child_path: &'static str) -> Self {
            Self { child_type, child_path }
        }
    }

    struct EmitOnceSession {
        child_type: Ty,
        child_path: &'static str,
        entry: Entry,
        done: bool,
    }

    impl ContentExtractor<Ty> for EmitOnce {
        fn create_session(&mut self, _: OwnedContentPtr<Ty>, _: &ExtractionContext) -> Option<Box<dyn ExtractionSession<Ty>>> {
            Some(Box::new(EmitOnceSession {
                child_type: self.child_type,
                child_path: self.child_path,
                entry: Entry::default(),
                done: false,
            }))
        }
    }

    impl ExtractionSession<Ty> for EmitOnceSession {
        fn advance(&mut self) -> Option<&Entry> {
            if self.done {
                return None;
            }
            self.done = true;
            self.entry.path.set_from_str(self.child_path);
            self.entry.size = 1;
            self.entry.skip_from_filtering = false;
            Some(&self.entry)
        }
        fn extract(&mut self) -> Option<Box<dyn Content<Ty>>> {
            Some(Box::new(BufferContent::<Ty>::with_content_type(b"x", self.child_path, self.child_type)))
        }
    }

    struct SliceExtractor;

    struct SliceSession {
        content: OwnedContentPtr<Ty>,
        offset: u64,
        length: u64,
        tag: u32,
        done: bool,
        entry: Entry,
    }

    impl ContentExtractor<Ty> for SliceExtractor {
        fn create_session(&mut self, content: OwnedContentPtr<Ty>, ec: &ExtractionContext) -> Option<Box<dyn ExtractionSession<Ty>>> {
            let tag = ec.params.and_then(|p| p.get::<u32>(var!("tag"))).unwrap_or(0);
            let length = ec.length.unwrap_or(content.size().saturating_sub(ec.offset));
            Some(Box::new(SliceSession {
                content,
                offset: ec.offset,
                length,
                tag,
                done: false,
                entry: Entry::default(),
            }))
        }
    }

    impl ExtractionSession<Ty> for SliceSession {
        fn advance(&mut self) -> Option<&Entry> {
            if self.done {
                return None;
            }
            self.done = true;
            let path = format!("t{}", self.tag);
            self.entry.path.set_from_str(&path);
            self.entry.size = self.length;
            self.entry.skip_from_filtering = false;
            Some(&self.entry)
        }
        fn extract(&mut self) -> Option<Box<dyn Content<Ty>>> {
            let n = self.length.min(u32::MAX as u64) as u32;
            let buf = self.content.read(self.offset, n)?.to_vec();
            let path = format!("t{}", self.tag);
            Some(Box::new(BufferContent::<Ty>::from_parts(buf, path, Some(Ty::SliceKid))))
        }
    }

    fn child_types(res: &ScanResult<Ty>, parent: ScanContentHandle) -> Vec<(String, Option<Ty>)> {
        let mut out = Vec::new();
        let mut c = res.child(parent);
        while let Some(h) = c {
            out.push((res.path(h).unwrap_or("?").to_string(), res.content_type(h)));
            c = res.next_sibling(h);
        }
        out
    }

    #[test]
    fn request_extract_runs_extractors_of_the_requested_type() {
        let mut scanner = ScannerBuilder::new()
            .add_analyzer(Ty::Root, 0, Request(Ty::Mid))
            .add_extractor(Ty::Mid, EmitOnce::new(Ty::MidKid, "mid"))
            .build();
        let mut content = BufferContent::<Ty>::with_content_type(b"root", "root", Ty::Root);
        let res = scanner.scan(&mut content, true);
        let root = res.root().unwrap();
        assert_eq!(child_types(&res, root), vec![("mid".into(), Some(Ty::MidKid))]);
        assert_eq!(res.objects_scanned(), 2);
    }

    #[test]
    fn request_extract_passes_offset_length_and_params() {
        let mut scanner = ScannerBuilder::new()
            .add_analyzer(Ty::Root, 0, RequestSlice)
            .add_analyzer(Ty::SliceKid, 0, Tag(1))
            .add_extractor(Ty::Slice, SliceExtractor)
            .build();
        let mut content = BufferContent::<Ty>::with_content_type(b"XXABCYY", "root", Ty::Root);
        let res = scanner.scan(&mut content, true);
        let root = res.root().unwrap();
        let kid = res.child(root).unwrap();
        assert_eq!(res.path(kid), Some("t9"));
        assert_eq!(res.content_type(kid), Some(Ty::SliceKid));
        let local = res.local(kid).unwrap();
        assert_eq!(local.get::<u32>(var!("tag")), Some(1));
        assert_eq!(local.get::<u32>(var!("size")), Some(3));
        assert_eq!(local.get::<u32>(var!("first")), Some(b'A' as u32));
    }

    #[test]
    fn sibling_requests_survive_a_nested_child_scan() {
        // Root queues Left then Right. Scanning Left's child (which itself
        // requests Deep) must not drop the still-pending Right request.
        let mut scanner = ScannerBuilder::new()
            .add_analyzer(Ty::Root, 0, RequestTwo(Ty::Left, Ty::Right))
            .add_analyzer(Ty::LeftKid, 0, Request(Ty::Deep))
            .add_extractor(Ty::Left, EmitOnce::new(Ty::LeftKid, "left"))
            .add_extractor(Ty::Right, EmitOnce::new(Ty::RightKid, "right"))
            .add_extractor(Ty::Deep, EmitOnce::new(Ty::DeepKid, "deep"))
            .build();
        let mut content = BufferContent::<Ty>::with_content_type(b"root", "root", Ty::Root);
        let res = scanner.scan(&mut content, true);
        let root = res.root().unwrap();
        assert_eq!(
            child_types(&res, root),
            vec![("left".into(), Some(Ty::LeftKid)), ("right".into(), Some(Ty::RightKid))]
        );
        let left = res.child(root).unwrap();
        assert_eq!(child_types(&res, left), vec![("deep".into(), Some(Ty::DeepKid))]);
        assert_eq!(res.objects_scanned(), 4);
    }

    #[test]
    fn three_level_request_extract_tree() {
        // Root --request Mid--> MidKid --request Inner--> InnerKid
        let mut scanner = ScannerBuilder::new()
            .add_analyzer(Ty::Root, 0, Request(Ty::Mid))
            .add_analyzer(Ty::MidKid, 0, Request(Ty::Inner))
            .add_analyzer(Ty::InnerKid, 0, Tag(3))
            .add_extractor(Ty::Mid, EmitOnce::new(Ty::MidKid, "mid"))
            .add_extractor(Ty::Inner, EmitOnce::new(Ty::InnerKid, "inner"))
            .build();
        let mut content = BufferContent::<Ty>::with_content_type(b"root", "root", Ty::Root);
        let res = scanner.scan(&mut content, true);

        let root = res.root().unwrap();
        assert_eq!(res.content_type(root), Some(Ty::Root));
        assert_eq!(res.path(root), Some("root"));
        assert_eq!(child_types(&res, root), vec![("mid".into(), Some(Ty::MidKid))]);

        let mid = res.child(root).unwrap();
        assert_eq!(res.parent(mid).map(|h| h.index), Some(root.index));
        assert_eq!(child_types(&res, mid), vec![("inner".into(), Some(Ty::InnerKid))]);

        let inner = res.child(mid).unwrap();
        assert_eq!(res.parent(inner).map(|h| h.index), Some(mid.index));
        assert!(res.child(inner).is_none());
        assert!(res.next_sibling(inner).is_none());
        assert_eq!(res.local(inner).and_then(|vm| vm.get::<u32>(var!("tag"))), Some(3));
        assert_eq!(res.objects_scanned(), 3);
    }

    #[test]
    fn drop_without_emit_does_not_queue_extraction() {
        let mut scanner = ScannerBuilder::new()
            .add_analyzer(Ty::Root, 0, DropWithoutEmit)
            .add_extractor(Ty::Slice, SliceExtractor)
            .build();
        let mut content = BufferContent::<Ty>::with_content_type(b"root", "root", Ty::Root);
        let res = scanner.scan(&mut content, true);
        let root = res.root().unwrap();
        assert!(res.child(root).is_none());
        assert_eq!(res.objects_scanned(), 1);
    }

    #[test]
    fn request_without_a_matching_extractor_is_a_no_op() {
        let mut scanner = ScannerBuilder::new().add_analyzer(Ty::Root, 0, RequestMissing).build();
        let mut content = BufferContent::<Ty>::with_content_type(b"root", "root", Ty::Root);
        let res = scanner.scan(&mut content, true);
        let root = res.root().unwrap();
        assert!(res.child(root).is_none());
        assert_eq!(res.objects_scanned(), 1);
    }
}

mod folder_symlinks {
    use crate::*;
    use std::fs;
    use std::path::{Path, PathBuf};

    #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ContentType)]
    #[repr(u16)]
    enum Ty {
        Folder,
    }

    struct TempDir(PathBuf);
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn symlink_file(original: &Path, link: &Path) -> std::io::Result<()> {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(original, link)
        }
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_file(original, link)
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = (original, link);
            Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "symlinks"))
        }
    }

    fn symlink_dir(original: &Path, link: &Path) -> std::io::Result<()> {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(original, link)
        }
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_dir(original, link)
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = (original, link);
            Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "symlinks"))
        }
    }

    fn setup() -> Option<TempDir> {
        let dir = TempDir(std::env::temp_dir().join(format!(
            "content_scan_symlink_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        )));
        fs::create_dir_all(dir.0.join("sub")).ok()?;
        fs::write(dir.0.join("real.txt"), b"hello").ok()?;
        fs::write(dir.0.join("sub").join("nested.txt"), b"in").ok()?;
        symlink_file(&dir.0.join("real.txt"), &dir.0.join("link_to_file")).ok()?;
        symlink_dir(&dir.0.join("sub"), &dir.0.join("link_to_dir")).ok()?;
        symlink_file(&dir.0.join("missing.txt"), &dir.0.join("dangling")).ok()?;
        Some(dir)
    }

    fn child_names(res: &ScanResult<Ty>) -> Vec<String> {
        let mut names = Vec::new();
        let Some(root) = res.root() else {
            return names;
        };
        let Some(first) = res.child(root) else {
            return names;
        };
        fn walk(res: &ScanResult<Ty>, handle: ScanContentHandle, names: &mut Vec<String>) {
            if let Some(p) = res.path(handle) {
                let name = p.rsplit(['\\', '/']).next().unwrap_or(p);
                names.push(name.to_string());
            }
            if let Some(child) = res.child(handle) {
                walk(res, child, names);
            }
            if let Some(sib) = res.next_sibling(handle) {
                walk(res, sib, names);
            }
        }
        walk(res, first, &mut names);
        names
    }

    fn scanned_names(root: &Path, recursive: bool) -> Vec<String> {
        let mut scanner = ScannerBuilder::new()
            .add_extractor(Ty::Folder, FolderExtractor::<Ty>::new(recursive, false))
            .build();
        let mut content = FolderContent::<Ty>::with_content_type(root, Ty::Folder);
        let res = scanner.scan(&mut content, false);
        child_names(&res)
    }

    #[test]
    fn directory_symlink_and_dangling_link_are_skipped() {
        let Some(dir) = setup() else {
            eprintln!("skipping folder_symlinks: cannot create symlinks on this host");
            return;
        };
        let names = scanned_names(&dir.0, true);
        assert!(names.contains(&"real.txt".into()), "{names:?}");
        assert!(names.contains(&"sub".into()), "{names:?}");
        assert!(names.contains(&"nested.txt".into()), "{names:?}");
        assert!(names.contains(&"link_to_file".into()), "{names:?}");
        assert!(!names.iter().any(|n| n == "link_to_dir"), "{names:?}");
        assert!(!names.iter().any(|n| n == "dangling"), "{names:?}");
    }

    #[test]
    fn non_recursive_walk_still_skips_directory_symlinks() {
        let Some(dir) = setup() else {
            eprintln!("skipping folder_symlinks: cannot create symlinks on this host");
            return;
        };
        let names = scanned_names(&dir.0, false);
        assert!(names.contains(&"real.txt".into()), "{names:?}");
        assert!(names.contains(&"link_to_file".into()), "{names:?}");
        assert!(!names.iter().any(|n| n == "sub"), "{names:?}");
        assert!(!names.iter().any(|n| n == "nested.txt"), "{names:?}");
        assert!(!names.iter().any(|n| n == "link_to_dir"), "{names:?}");
        assert!(!names.iter().any(|n| n == "dangling"), "{names:?}");
    }
}

mod dependencies_derive {
    use crate::Dependencies;

    #[derive(Dependencies)]
    #[Dependencies(name = "xyz", requires = "abc")]
    struct PluginA;

    #[derive(Dependencies)]
    #[Dependencies(name = "xyz", requires = ["abc", "123", "blablabla"])]
    struct PluginB;

    #[derive(Dependencies)]
    #[Dependencies(name = "solo")]
    struct PluginC;

    #[test]
    #[cfg(debug_assertions)]
    fn single_requires_string() {
        assert_eq!(PluginA{}.name(), "xyz");
        assert_eq!(PluginA{}.dependencies(), &["abc"]);
    }

    #[test]
    #[cfg(debug_assertions)]
    fn requires_array() {
        assert_eq!(PluginB{}.name(), "xyz");
        assert_eq!(PluginB{}.dependencies(), &["abc", "123", "blablabla"]);
    }

    #[test]
    #[cfg(debug_assertions)]
    fn requires_optional() {
        assert_eq!(PluginC{}.name(), "solo");
        assert_eq!(PluginC{}.dependencies(), &[] as &[&str]);
    }
}
