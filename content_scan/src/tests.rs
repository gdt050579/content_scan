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

mod filter {
    use crate::{ContentPath, Filter, FilterBuilder, Precedence};

    fn process(filter: &Filter, path: &str) -> bool {
        filter.should_process(&ContentPath::from_str(path), 1)
    }

    #[test]
    fn higher_precedence_exclude_overrides_earlier_include() {
        let filter = FilterBuilder::new()
            .include_extensions(Precedence::Low, &["txt"])
            .exclude_extensions(Precedence::Highest, &["txt"])
            .allow_the_rest()
            .build();
        assert!(!process(&filter, "notes.txt"));
        assert!(process(&filter, "notes.rs"));
    }

    #[test]
    fn higher_precedence_include_overrides_earlier_exclude() {
        let filter = FilterBuilder::new()
            .exclude_extensions(Precedence::Low, &["txt"])
            .include_extensions(Precedence::High, &["txt"])
            .deny_the_rest()
            .build();
        assert!(process(&filter, "notes.txt"));
        assert!(!process(&filter, "notes.rs"));
    }

    #[test]
    fn same_precedence_keeps_insertion_order() {
        let include_first = FilterBuilder::new()
            .include_extensions(Precedence::Medium, &["txt"])
            .exclude_extensions(Precedence::Medium, &["txt"])
            .deny_the_rest()
            .build();
        assert!(process(&include_first, "notes.txt"));

        let exclude_first = FilterBuilder::new()
            .exclude_extensions(Precedence::Medium, &["txt"])
            .include_extensions(Precedence::Medium, &["txt"])
            .allow_the_rest()
            .build();
        assert!(!process(&exclude_first, "notes.txt"));
    }
}

