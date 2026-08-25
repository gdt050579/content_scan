# Folder

A directory is not a byte stream. `FolderContent` exists so the scanner has an object it can dispatch on; `FolderExtractor` turns that object into children. Together they walk the filesystem with the same pipeline as a ZIP or any other container: filter → identify → analyze → extract, up to [`max_depth`](../chapter-3/recursion.md).

You supply the variant. The crate does not define `Folder` for you.

```rust
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ContentType)]
#[repr(u16)]
enum MyTypes {
    Folder,
    // Png, Zip, …
}

let mut scanner = ScannerBuilder::<MyTypes>::new()
    .max_depth(64)
    .add_extractor(MyTypes::Folder, FolderExtractor::<MyTypes>::new(true, false))
    // identifiers + analyzers for the files you care about
    .build();

let mut root = FolderContent::<MyTypes>::with_content_type("./src", MyTypes::Folder);
let result = scanner.scan(&mut root, false);
```

Register the extractor for **the same variant** the `FolderContent` is pinned to. The scanner sees `content_type() == Some(Folder)` and never runs identifiers on the directory itself.

## `FolderContent`

```rust
FolderContent::<MyTypes>::with_content_type(path, MyTypes::Folder);
```

`path` is `impl AsRef<Path>` → [`ContentPath::from_os`](../chapter-2/content_path.md), so a non-UTF-8 directory name stays openable.

| Method | Behaviour |
| --- | --- |
| `content_type()` | Always `Some(the variant you passed)`. |
| `path()` | The directory. |
| `size()` | `0`. |
| `read(...)` | Always `None`. |

A generic analyzer that hashes bytes should skip folders (`content.content_type() == Some(MyTypes::Folder)`), as the md5 / sha1 / entropy examples do.

## `FolderExtractor`

```rust
FolderExtractor::<MyTypes>::new(recursive, open_files_exclusively)
```

| Flag | Meaning |
| --- | --- |
| `recursive` | `true` — emit subdirectories (as `FolderContent` of the **same** type, so this extractor runs again). `false` — only files directly inside the parent; nested folders are skipped. |
| `open_files_exclusively` | Forwarded to [`FileContent::with_size`](../chapter-2/content.md). `true` — mmap + exclusive lock. `false` — shared LRU (usual choice when other processes might have the file open). |

`create_session` calls `read_dir` on the parent’s path. If that fails, or if the parent has no pinned `content_type()`, it returns `None` and this extractor is skipped.

Configuration lives on the extractor; the `ReadDir` cursor and current [`Entry`](../chapter-2/extractor.md) live on the **session**, so a nested folder can open another session without clobbering the parent walk.

## What each entry becomes

| Directory entry | Child `Content` | Notes |
| --- | --- | --- |
| Regular file | `FileContent::with_size(path, len, exclusive)` | Size comes from the dirent (or from the symlink **target**). No extra `stat` at construction. |
| Subdirectory | `FolderContent` with the **parent’s** type | Only if `recursive`. `Entry::skip_from_filtering = true`. |
| Unreadable / unknown `file_type` | skipped | The rest of the directory is still enumerated. |

Paths are filled with `ContentPath::set_from_os` on one reused `Entry`, so non-UTF-8 names survive the walk.

Files are **not** pinned: `FileContent` is constructed without a content type, so identifiers run (magic, name, extension). That is how a folder walk finds PNGs and ZIPs.

## `filter_root` and `skip_from_filtering`

Pass **`scan(..., false)`** for a `FolderContent` root when the filter is written for files (extensions, names). A directory has no `.png` suffix; `filter_root = true` would reject the root and visit nothing. Children are still filtered. See [Recursion and filter_root](../chapter-3/recursion.md).

Subdirectory entries set `skip_from_filtering` so an extension filter does not block descent. Files in those folders are still tested. That is “only `*.rs`, recursively.”

An [observer](../chapter-3/observer.md) sees kept files as `on_extraction`, rejected files as `on_filtered`, and folders as `on_scan_object` (they were not filtered).

## Symlinks and junctions

The walk inspects the **link entry**, not only `FileType::is_symlink` (on Windows that misses junctions and other reparse points).

| Link target | Behaviour |
| --- | --- |
| Directory (or junction) | **Skipped.** Prevents cycles. |
| Regular file | **Followed.** Emitted as `FileContent` with the target’s size. |
| Dangling / unreadable target | **Skipped.** |

A symlink to a file is scanned like any other file (identifiers see the target bytes once `FileContent` opens it). A symlink to a folder is not entered.

## Depth

[`max_depth`](../chapter-3/recursion.md) counts **directory nesting** (plus the file as the last step). Default `8` is shallow for a real tree; the md5 / finder examples use `64`. Hitting the cap is silent: the folder is still recorded and analyzed; no further children.

`recursive: false` is a different knob: it never emits subdirectory objects at all, even when depth remains.

## File or directory at the call site

The usual `main` is the same shape as [Recursion](../chapter-3/recursion.md):

```rust
let res = if Path::new(&path).is_dir() {
    let mut content = FolderContent::<MyTypes>::with_content_type(&path, MyTypes::Folder);
    scanner.scan(&mut content, false)
} else {
    let mut content = FileContent::<MyTypes>::new(&path, false);
    scanner.scan(&mut content, true)
};
```

One scanner instance, two roots. Do not wrap a file in `FolderContent` or a directory in `FileContent`.

## With ZIP

Register both extractors. A folder walk emits files; `ZipIdentifier` may type some of them as `Zip`; `ZipExtractor` then unpacks members as further children. `max_depth` covers both kinds of nesting. See [ZIP](zip.md) and the `find_zip` / `zip_png_size` examples.
