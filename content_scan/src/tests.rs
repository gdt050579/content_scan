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
        (start..end)
            .map(|i| unsafe { list.get(i).clone() })
            .collect()
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
        let mut list = AnalyzerList::new(
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
        let list = AnalyzerList::new(
            vec![typed(0, 0, "a"), typed(2, 0, "c")],
            TestType::COUNT,
        );
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
        let mut list = AnalyzerList::new(
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
        let mut list = AnalyzerList::new(vec![generic(1, "g")], 0);
        assert_eq!(list.len(), 1);
        assert_eq!(list.range(TestType::A), None);
        assert_eq!(list.generic_range(), Some((0, 1)));
        assert_eq!(unsafe { list.get(0) }, "g");
    }

    #[test]
    fn typed_then_generics() {
        let mut list = AnalyzerList::new(
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
        let mut list = AnalyzerList::new(
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

mod extraction_pool {
    use crate::extraction_pool::{ExtractionHandle, ExtractionPool};

    #[test]
    fn acquire_assigns_monotonic_uids_and_growing_indices() {
        let mut pool = ExtractionPool::new(2);
        let a = pool.acquire_slot(10u32);
        let b = pool.acquire_slot(20u32);
        assert_eq!(a.index(), 0);
        assert_eq!(a.uid(), 1);
        assert_eq!(b.index(), 1);
        assert_eq!(b.uid(), 2);
        assert_eq!(pool.get(a), Some(&10));
        assert_eq!(pool.get(b), Some(&20));
    }

    #[test]
    fn concurrent_slots_are_independent() {
        let mut pool = ExtractionPool::new(2);
        let a = pool.acquire_slot("a");
        let b = pool.acquire_slot("b");
        assert_ne!(a.index(), b.index());
        assert_ne!(a.uid(), b.uid());
        assert_eq!(pool.get(a), Some(&"a"));
        assert_eq!(pool.get(b), Some(&"b"));
    }

    #[test]
    fn release_makes_get_return_none() {
        let mut pool = ExtractionPool::new(1);
        let h = pool.acquire_slot(7u8);
        pool.release_slot(h);
        assert_eq!(pool.get(h), None);
        assert_eq!(pool.get_mut(h), None);
    }

    #[test]
    fn released_index_is_reused_with_new_uid() {
        let mut pool = ExtractionPool::new(1);
        let first = pool.acquire_slot(1u32);
        assert_eq!(first.index(), 0);
        assert_eq!(first.uid(), 1);
        pool.release_slot(first);

        let second = pool.acquire_slot(2u32);
        assert_eq!(second.index(), first.index());
        assert_eq!(second.uid(), 2);
        assert_ne!(second.uid(), first.uid());
        assert_eq!(pool.get(first), None);
        assert_eq!(pool.get(second), Some(&2));
    }

    #[test]
    fn double_release_is_noop() {
        let mut pool = ExtractionPool::new(1);
        let h = pool.acquire_slot(1u32);
        pool.release_slot(h);
        pool.release_slot(h);
        let h2 = pool.acquire_slot(9u32);
        assert_eq!(h2.index(), h.index());
        assert_eq!(pool.get(h2), Some(&9));
    }

    #[test]
    fn release_with_stale_uid_does_not_free_live_slot() {
        let mut pool = ExtractionPool::new(1);
        let stale = pool.acquire_slot(1u32);
        pool.release_slot(stale);
        let live = pool.acquire_slot(2u32);
        assert_eq!(live.index(), stale.index());
        assert_ne!(live.uid(), stale.uid());
        pool.release_slot(stale);
        assert_eq!(pool.get(live), Some(&2));
    }

    #[test]
    fn get_and_release_out_of_bounds_handle() {
        let mut pool = ExtractionPool::<u32>::new(1);
        let _ = pool.acquire_slot(1);
        let bogus = ExtractionHandle::new(99, 1);
        assert_eq!(bogus.index(), 99);
        assert_eq!(bogus.uid(), 1);
        assert_eq!(pool.get(bogus), None);
        assert_eq!(pool.get_mut(bogus), None);
        pool.release_slot(bogus); // must not panic
    }

    #[test]
    fn get_with_wrong_uid_returns_none() {
        let mut pool = ExtractionPool::new(1);
        let h = pool.acquire_slot(5u32);
        let wrong = ExtractionHandle::new(h.index(), h.uid().wrapping_add(1));
        assert_eq!(wrong.index(), h.index());
        assert_ne!(wrong.uid(), h.uid());
        assert_eq!(pool.get(wrong), None);
        assert_eq!(pool.get_mut(wrong), None);
        assert_eq!(pool.get(h), Some(&5));
    }

    #[test]
    fn release_with_wrong_uid_is_noop() {
        let mut pool = ExtractionPool::new(1);
        let h = pool.acquire_slot(5u32);
        pool.release_slot(ExtractionHandle::new(h.index(), 0));
        assert_eq!(pool.get(h), Some(&5));
    }

    #[test]
    fn get_mut_allows_mutation() {
        let mut pool = ExtractionPool::new(1);
        let h = pool.acquire_slot(String::from("old"));
        *pool.get_mut(h).unwrap() = String::from("new");
        assert_eq!(pool.get(h), Some(&String::from("new")));
    }

    #[test]
    fn handle_new_and_accessors() {
        let h = ExtractionHandle::new(3, 7);
        assert_eq!(h.index(), 3);
        assert_eq!(h.uid(), 7);
        assert_eq!(h, ExtractionHandle::new(3, 7));
        assert_ne!(h, ExtractionHandle::new(3, 8));
        assert_ne!(h, ExtractionHandle::new(2, 7));
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
    }

    struct TaggedId;
    impl ContentIdentifier<Ty> for TaggedId {
        fn identify_method(&self) -> Option<IdentifyMethod> {
            Some(IdentifyMethod::Magic(b"TAG"))
        }
        fn validate(&self, _: &dyn Content<Ty>) -> bool {
            true
        }
    }

    struct TaggedButRejectsId;
    impl ContentIdentifier<Ty> for TaggedButRejectsId {
        fn identify_method(&self) -> Option<IdentifyMethod> {
            Some(IdentifyMethod::Magic(b"TAG"))
        }
        fn validate(&self, _: &dyn Content<Ty>) -> bool {
            false
        }
    }

    /// Example: no `IdentifyMethod` — decide from the path in `validate`.
    struct CustomPathId;
    impl ContentIdentifier<Ty> for CustomPathId {
        fn identify_method(&self) -> Option<IdentifyMethod> {
            None
        }
        fn validate(&self, content: &dyn Content<Ty>) -> bool {
            content.path().as_printable_string().ends_with(".custom")
        }
    }

    /// Example: catch-all custom identifier (any non-empty payload).
    struct NonEmptyId;
    impl ContentIdentifier<Ty> for NonEmptyId {
        fn identify_method(&self) -> Option<IdentifyMethod> {
            None
        }
        fn validate(&self, content: &dyn Content<Ty>) -> bool {
            content.size() > 0
        }
    }

    fn identified_type(scanner: &mut Scanner<Ty>, buf: &[u8], path: &str) -> Option<Ty> {
        let mut content = BufferContent::<Ty>::new(buf, path);
        let res = scanner.scan(&mut content, true);
        res.root().and_then(|h| res.content_type(h))
    }

    #[test]
    fn custom_identifier_classifies_when_identify_method_is_none() {
        let mut scanner = ScannerBuilder::new()
            .add_identifier(Ty::Custom, CustomPathId)
            .build();
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
}

mod max_depth {
    use crate::*;

    #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ContentType)]
    #[repr(u16)]
    enum Ty {
        Nest,
    }

    /// Emits one smaller child so scans form a single chain of nested objects.
    struct NestExtractor {
        pool: ExtractionPool<bool>,
        entry: Entry,
    }
    impl Default for NestExtractor {
        fn default() -> Self {
            Self {
                pool: ExtractionPool::new(4),
                entry: Entry::default(),
            }
        }
    }
    impl ContentExtractor<Ty> for NestExtractor {
        fn acquire(&mut self, content: &mut dyn Content<Ty>, _: &mut VarMap) -> Option<ExtractionHandle> {
            if content.size() == 0 {
                return None;
            }
            Some(self.pool.acquire_slot(false))
        }
        fn advance(&mut self, handle: ExtractionHandle, content: &mut dyn Content<Ty>) -> Option<&Entry> {
            let done = self.pool.get_mut(handle)?;
            if *done {
                return None;
            }
            *done = true;
            self.entry.path.set_from_str("child");
            self.entry.size = content.size().saturating_sub(1);
            self.entry.skip_from_filtering = false;
            Some(&self.entry)
        }
        fn extract(&mut self, _: ExtractionHandle, content: &mut dyn Content<Ty>) -> Option<Box<dyn Content<Ty>>> {
            let n = content.size().saturating_sub(1) as usize;
            Some(Box::new(BufferContent::<Ty>::with_content_type(&vec![0u8; n], "child", Ty::Nest)))
        }
        fn release(&mut self, handle: ExtractionHandle) {
            self.pool.release_slot(handle);
        }
    }

    fn scanned(max_depth: u32) -> u32 {
        let mut scanner = ScannerBuilder::new()
            .max_depth(max_depth)
            .add_extractor(Ty::Nest, NestExtractor::default())
            .build();
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

