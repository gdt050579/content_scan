# ZIP

Two plugins: `ZipIdentifier` decides that an object **is** a ZIP; `ZipExtractor` unpacks its members as child [`Content`](../chapter-2/content.md). Pair them on the same variant of **your** enum.

```rust
let mut scanner = ScannerBuilder::<MyTypes>::new()
    .add_identifier(MyTypes::Zip, ZipIdentifier::new())
    .add_extractor(MyTypes::Zip, ZipExtractor::new())
    .build();
```

Identification is by **content**, not by `.zip` extension. A renamed archive still matches; a file that merely starts with `PK\x03\x04` does not, unless the tail looks like a real End of Central Directory.

## `ZipIdentifier`

Fast check: [`IdentifyMethod::Magic`](../chapter-2/identifier.md) `PK\x03\x04` (local-file header). That is four bytes, well inside the scanner’s 16-byte magic window.

`validate` then looks for an **EOCD** signature `PK\x05\x06` in the tail, with a comment length that lands exactly at `size`:

1. Reject if `size < 22` (minimum EOCD).
2. Search the last `min(512, size)` bytes.
3. If that misses, search up to the last **65557** bytes (22-byte EOCD + 65535-byte comment).

A buffer that starts with a ZIP local-file magic but has no EOCD (truncated download, random `PK` prefix) is **not** typed as ZIP. Typed analyzers and the ZIP extractor will not run; generic analyzers still will.

There is no `ZipIdentifier` per extension. If you only want `*.zip` names, put that in a [filter](../chapter-3/filter.md), or write your own identifier. The built-in one is for “this is a ZIP archive.”

`new()` / `Default` are equivalent.

## `ZipExtractor`

`create_session` wraps the parent in a [`ContentReader`](builtins.md#contentreader) and opens `zip::ZipArchive`. If that fails, it returns `None` (this extractor is skipped; another extractor for the same type could still run).

The parent can be a `FileContent`, a `BufferContent`, or any other `Content` — the reader only needs `read` / `size`. Nested ZIPs work: a member that is itself a ZIP is a new object; after identification the same extractor opens another **session** on that child. Do not store the `ZipArchive` on the extractor itself.

### Members

Directory entries **inside** the archive are skipped. Regular files become children of the ZIP object:

| Member size | Child |
| --- | --- |
| `< 1 MiB` (`0x100000`) | `BufferContent` (decompressed into memory) |
| `≥ 1 MiB` | Decompressed to a unique file under `std::env::temp_dir()`, wrapped in `FileContent::with_size(..., exclusive = false)` |

Large members are **not** deleted automatically when the scan ends. The temp name is `content_scan_zip_{pid}_{n}.tmp`. If you unpack many huge archives, plan cleanup (or use a custom extractor).

Paths come from `zip::ZipFile::enclosed_name()` (zip-slip safe). If that is `None`, the entry path is empty. `skip_from_filtering` is **false**: members are subject to the filter like any other child.

The result tree under a ZIP is **flat**: `archive/a/b.png` is one child of the ZIP node, not a nested folder object. [Navigating ScanResult](../chapter-4/scan_result.md) still uses parent / child / sibling; there is just no `FolderContent` layer for paths inside the archive. ZIP-in-ZIP **does** nest: inner archive is a child, its members are grandchildren.

### `ExtractionContext`

The built-in extractor currently **ignores** [`ExtractionContext`](../chapter-2/extraction_context.md). `ContentReader` always starts at offset `0` of the parent. That is correct when the whole object **is** the ZIP (identified as `Zip`, or a file you passed in).

It is **not** enough for `request_extract(MyTypes::Zip).at(offset)` on a PE overlay: the archive would be parsed from the start of the PE, not from `offset`. For an embedded ZIP, emit a child `Content` that is already a view of that window, or write an extractor that copies `ctx.offset` / `ctx.length` into the session (the README sketch of a custom ZIP session).

## Filters and `filter_root`

When the **root** is the archive and the filter keeps only `*.png` members, pass `scan(..., false)`. The path `photos.zip` would fail an extension filter; members are still filtered. Same idea as a [folder](folder.md) root.

`zip_png_size` is that pattern: ZIP identify + extract, PNG identifier/analyzer, `include_extensions(..., &["png"])`, `filter_root = false`.

## Folder walk without unpacking

`find_zip` registers `ZipIdentifier` and `FolderExtractor` but **not** `ZipExtractor`. The walk emits files; ZIP ones are typed and a small analyzer prints them; members are never opened. Cheaper when you only need to locate archives.

To scan **inside** every ZIP in a tree, register both extractors. `max_depth` must cover directory nesting **plus** archive nesting.

```rust
ScannerBuilder::<MyTypes>::new()
    .max_depth(64)
    .add_identifier(MyTypes::Zip, ZipIdentifier::new())
    .add_extractor(MyTypes::Zip, ZipExtractor::new())
    .add_extractor(MyTypes::Folder, FolderExtractor::<MyTypes>::new(true, false))
    .build();
```

## What you still write

ZIP built-ins do not analyze members. A PNG inside an archive is just `Content` until you add a PNG identifier and analyzer. The extractor’s job is to produce children; [Chapter 2](../chapter-2/analyzer.md) is what you do with them.
