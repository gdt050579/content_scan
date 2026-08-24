# Recursion and filter_root

Two settings control how deep a scan goes and whether the **root** object is subject to the [filter](filter.md): `max_depth` on the builder, and the `filter_root` flag on `scan()`.

## `max_depth`

```rust
ScannerBuilder::<MyTypes>::new()
    .max_depth(16)
    .build();
```

Default is **8**. The value is clamped to `1..=u32::MAX - 2`.

The root is **depth 1**. A child extracted from a depth-`N` object is depth `N + 1`. Extraction **does not start** when that next child would exceed `max_depth`: `max_depth(8)` visits at most eight objects on any path from the root.

For a directory walk, depth is nesting of folders (plus files as the last step). For ZIP-in-ZIP, it is nesting of archives. One `max_depth` covers every extractor.

Hitting the cap is silent: the parent is still analyzed; no further children are opened. Raise it for deep trees (`md5` example uses 64). Depth `1` means “this object only, never extract.”

## `filter_root`

```rust
scanner.scan(&mut content, true);  // test the root against the filter
scanner.scan(&mut folder, false);  // do not test the root
```

The flag is **only about the root**. Extracted children are filtered either way, except when `Entry::skip_from_filtering` is set.

Pass **`true`** for a normal file: if the filter rejects it, `scan()` returns immediately with an empty result (no objects).

Pass **`false`** when the root is a **container the filter was never written to accept**:

- A `FolderContent` scanned with `include_extensions(..., &["png"])` — a directory has no `.png` name.
- A ZIP scanned with a filter that keeps only `*.png` members — the archive path is `photos.zip`.

If you pass `true` in those cases, the scan is empty and nothing inside is visited.

`FolderExtractor` marks subdirectory entries `skip_from_filtering` so the walk can descend even when the filter is extension-based. Files in those folders are still filtered. That is how “only PNGs, recursively” works.

## Putting them together

```rust
let mut scanner = ScannerBuilder::<ImageType>::new()
    .filter(
        FilterBuilder::new()
            .include_extensions(Precedence::Medium, &["png", "jpg", "jpeg"])
            .deny_the_rest()
            .build(),
    )
    .max_depth(32)
    .add_extractor(ImageType::Folder, FolderExtractor::<ImageType>::new(true, false))
    // identifiers + analyzers ...
    .build();

let result = if path.is_dir() {
    let mut root = FolderContent::<ImageType>::with_content_type(&path, ImageType::Folder);
    scanner.scan(&mut root, false) // folder name is not an image extension
} else {
    let mut file = FileContent::<ImageType>::new(&path, false);
    scanner.scan(&mut file, true)
};
```

Reuse this `scanner` for every path in the batch. Depth and filter stay configured; only the context is reset — [create once, scan many times](scanner.md#create-once-scan-many-times).
