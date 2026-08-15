# Issues

Local review of `content_scan` 0.1.3 (library, proc-macro, examples). Ranked by what will actually break scans or violate Rust aliasing / UTF-8 rules.

## Tracker

Check an item when it is fixed.

**Critical**
- [x] 1. `ScanResult::path` UTF-8 UB
- [ ] 2. Overlapping `&mut` to the same extractor (aliasing UB)

**High**
- [x] 3. Filter `Precedence` is a no-op
- [x] 4. Identifiers with `identify_method() == None` never run
- [x] 5. `max_depth` is off by one
- [x] 6. Folder walk dies on the first I/O error
- [x] 7. Magic matcher semantics flip by pattern count
- [x] 8. `VarMap` pool is cleared, not recycled
- [x] 9. `FileContent` opens Exclusive

**Medium**
- [ ] 10. Directory-symlink skip is ineffective on Unix
- [x] 11. Extension and file-name rules are case-sensitive
- [ ] 12. Magic identification is single-candidate
- [x] 13. `extract()` VarMap is not scoped to the parent session (FAD)
- [x] 14. `COUNT: u16` cannot represent 65536 variants (FAD)
- [x] 15. Proc-macro taken from crates.io, not the workspace path (FAD)
- [ ] 16. No integration tests for the scan pipeline

**Low / hygiene**
- [x] 17. `next_siblig_index` typo
- [x] 18. Proc-macro leftover from varmap
- [x] 19. Examples depend on `varmap = "*"`
- [ ] 20. Doctests are `ignore`
- [x] 21. `image_size` filter misses `jpeg` / `JPG`
- [x] 22. Workspace release profile sets `debug = true`
- [x] 23. Romanian comments in the matcher
- [x] 24. `FolderExtractor` session fields are not in the pool (FAD)
- [ ] 25. `IntoContentPath` is incomplete
- [x] 26. Temporary junk file in the crate directory

---

## Critical

### 1. `ScanResult::path` UTF-8 UB — fixed

**Where:** `content_scan/src/context.rs`, `content_scan/src/scanner.rs`

The scanner now interns `ContentPath::as_printable_string()` (always valid UTF-8). `ScanResult::path` uses `from_utf8` instead of `from_utf8_unchecked`. README and rustdoc were still describing interned `as_bytes()`; those now match the code.

Identification and filtering still use `as_bytes()` (correct: match on the faithful path bytes).

### 2. Overlapping `&mut` to the same extractor (aliasing UB)

**Where:** `content_scan/src/scanner.rs` (`extract_content`), `content_scan/src/plugin_list.rs` (`PluginsList::get`)

`PluginsList::get` is an unchecked exclusive borrow. `extract_content` keeps that `&mut` live while calling `get` again and while `inner_scan` mutably borrows the same list for nested extraction.

DFS extract-before-recurse happens to work today, but stacked borrows / Miri treat this as instant UB.

**Fix:** Do not hold a plugin reference across recursion. Index into the list, or use raw pointers with a documented invariant, or restructure so `advance` / `extract` / `inner_scan` never overlap exclusive borrows of the same slot.

---

## High

### 3. Filter `Precedence` is a no-op — fixed

**Where:** `content_scan/src/filter.rs` (`ReadyFilterBuilder::build`)

`build` now sorts rules from `Highest` to `Lowest` (stable, so insertion order is kept within a bucket). README matches that contract.

### 4. Identifiers with `identify_method() == None` never run — fixed

**Where:** `content_scan/src/scanner.rs` (`retrieve_content_type`), `content_scan/src/identifier_set.rs`

Identifiers that return `None` from `identify_method` are kept in registration order and tried via `validate()` after magic / name / extension. README and rustdoc describe that fallback.

### 5. `max_depth` is off by one — fixed

**Where:** `content_scan/src/scanner.rs` (`extract_content`)

Extraction is skipped when `depth >= max_depth`, so the next child cannot exceed the limit. `max_depth(8)` visits at most eight objects on a path. README matches that contract.

### 6. Folder walk dies on the first I/O error — fixed

**Where:** `content_scan/src/implementations/folder_extractor.rs` (`advance`)

`advance` now `continue`s on `Err` from `ReadDir::next` and `file_type()`. Only an exhausted iterator returns `None`. Rustdoc and README note that unreadable entries are skipped.

### 7. Magic matcher semantics flip by pattern count — fixed

**Where:** `content_scan/src/matcher.rs`, `matcher/fast_magic.rs`, `matcher/trie.rs`

FastMagic `starts_with` now tries length 4, then 3, then 2. The Trie already kept the longest prefix. Overlapping magics resolve the same way regardless of whether the builder picks FastMagic or Trie.

### 8. `VarMap` pool is cleared, not recycled — fixed

**Where:** `content_scan/src/context.rs` (`Context::clear`)

`clear` no longer drops the pool. It `truncate(128)`s, clears each remaining `VarMap`, and resets `used_local_varmaps`. README’s reuse-across-scans claim now matches the code.

### 9. `FileContent` opens Exclusive — fixed

**Where:** `content_scan/src/implementations/file_content.rs`

Exclusive mmap is now opt-in via an `exclusive` flag on `FileContent` constructors (`true` = mmap + exclusive lock, `false` = shared LRU). `FolderExtractor::new(recursive, open_files_exclusively)` forwards that flag. README and rustdoc describe both modes.

---

## Medium

### 10. Directory-symlink skip is ineffective on Unix

**Where:** `content_scan/src/implementations/folder_extractor.rs`

Unix `DirEntry::file_type()` does not follow symlinks, so `is_dir() && is_symlink()` almost never fires. A symlink to a directory is emitted as `FileContent`, not skipped and not walked. Docs claim directory symlinks are skipped to prevent cycles.

**Fix:** Treat `is_symlink()` as a skip for directories after a follow-or-not policy, or use `symlink_metadata` / `path.is_dir()` consistently and document whether links are followed.

### 11. Extension and file-name rules are case-sensitive — fixed

**Where:** `content_scan/src/filter.rs`, `content_scan/src/utils.rs`

Filter extension and file-name rules are ASCII case-insensitive: the path basename / extension is lowercased at match time, and registered patterns are lowercased when the filter is built. `Photo.JPG` is accepted by a filter that allows `jpg`. Identification via `IdentifyMethod::Extension` / `Name` is still case-sensitive.

### 12. Magic identification is single-candidate

**Where:** `content_scan/src/scanner.rs` (`retrieve_content_type`)

If the chosen magic fails `validate()`, overlapping magics are never tried. The scanner falls through to file name / extension only.

**Fix:** Return all magic hits (or iterate remaining matchers) and validate each until one accepts.

### 13. `extract()` VarMap is not scoped to the parent session — FAD

**Where:** `content_scan/src/context.rs`, `content_scan/src/scanner.rs`

`clear_extract()` runs at the start of every `inner_scan`, including children. Accepted as designed: `extract()` is an analyzer-to-extractor channel for the *current* object (an analyzer records e.g. an embedded ZIP offset; a generic extractor reads it in `acquire`), not parent-session scratch. Nested scans get a fresh map. Copy hints into the extraction session during `acquire`.

### 14. `COUNT: u16` cannot represent 65536 variants — FAD

**Where:** `content-scan-proc-macro/src/derive.rs`, README

Docs allow 65536 variants while `COUNT` is `u16` (max 65535). Accepted as designed: more than 64k distinct content types is not a realistic use of this scanner.

### 15. Proc-macro taken from crates.io, not the workspace path — FAD

**Where:** `content_scan/Cargo.toml`

Depends on `content_scan_proc_macro = "0.1.1"` from crates.io, with the workspace path dep commented out. Accepted as designed: the path is switched on only while working on the proc-macro; otherwise the published crates.io crate is used.

### 16. No integration tests for the scan pipeline

**Where:** `content_scan/src/tests.rs`

Tests cover `PluginsList`, `ExtractionPool`, `utils`, and matchers. There are none for `Scanner`, `Filter`, `ContentPath`, `FileContent`, or `FolderExtractor`.

**Fix:** Add tests for filter precedence, `max_depth`, custom identifiers, folder walk errors, and `ScanResult` paths.

---

## Low / hygiene

### 17. `next_siblig_index` typo — fixed

**Where:** `content_scan/src/object.rs`, `context.rs`, `scanner.rs`

The field is `next_sibling_index` everywhere (`object.rs`, `context.rs`, `scanner.rs`). Public API and README already use `next_sibling`. The last leftover was a `scanner.rs` comment (`siblig`), now corrected.

### 18. Proc-macro leftover from varmap — fixed

**Where:** `content-scan-proc-macro`

The derive entry point is `derive_content_type` / `process_content_type`. Crate description and keywords no longer mention varmap (`content-scan`, `proc-macro`, `derive`).

### 19. Examples depend on `varmap = "*"` — fixed

**Where:** `examples/Cargo.toml`

The wildcard `varmap` dependency is gone. Examples depend only on `content_scan` (path) and get `var!` / `VarMap` through its re-export.

### 20. Doctests are `ignore`

**Where:** `content_scan/src/lib.rs`, README examples

Crate doctests and many README snippets are not compiled, so they can rot.

### 21. `image_size` filter misses `jpeg` / `JPG` — fixed

**Where:** `examples/image_size/main.rs`

The example filter now allows `jpg` and `jpeg`. Uppercase `.JPG` is accepted as well because extension rules are ASCII case-insensitive (issue 11).

### 22. Workspace release profile sets `debug = true` — fixed

**Where:** root `Cargo.toml`

The `[profile.release]` `debug = true` override is gone. Release builds use Cargo’s default (no debug info).

### 23. Romanian comments in the matcher — fixed

**Where:** `content_scan/src/scanner.rs`, `content_scan/src/matcher.rs`

Comments are English (`type-specific` / `generic` extractors; FastMagic vs trie).

### 24. `FolderExtractor` session fields are not in the pool — FAD

**Where:** `content_scan/src/implementations/folder_extractor.rs`

`entry` and `current_is_folder` live on the extractor, not in `ExtractionPool`. Accepted as designed: the scanner’s DFS (extract before recurse) keeps a single in-flight entry, so pooling those fields is unnecessary.

### 25. `IntoContentPath` is incomplete

**Where:** `content_scan/src/content_path.rs`

Implements `&str`, `&Path`, `ContentPath`. Missing `String`, `PathBuf`, `&String`.

### 26. Temporary junk file in the crate directory — fixed

**Where:** `content_scan/FAR56D4.tmp`

The leftover temp file is gone; ignored / considered fixed.

---

## What holds up

The plugin split (identify / analyze / extract), `ExtractionPool` generation counters, `skip_from_filtering` for folder descent, and `ContentPath`’s lossless OS-path handling are the right shape for this library. The issues above are mostly in the last mile: unsafe UTF-8, borrow workarounds, and docs that describe behavior the builder never implements.
