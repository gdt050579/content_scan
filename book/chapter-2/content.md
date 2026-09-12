# Content

A scan is a walk over **content objects**. Each object is one thing the scanner can name, measure, and (usually) read: a file, an in-memory buffer, a directory, a ZIP member, a decoded Base64 blob. Plugins never see a raw `Vec<u8>` or an `std::fs::File`. They see `&mut dyn Content<T>`.

`T` is your [`ContentType`](content_type.md) enum. The same `Content` trait is used for the root you pass to `scan()` and for every child an extractor yields.

Two child pages cover the type parameter and the name of an object:

- [`ContentType`](content_type.md) — the closed set of kinds *this* scanner understands.
- [`ContentPath`](content_path.md) — the path or synthetic address attached to each object.

## The trait

```rust
pub trait Content<T: ContentType> {
    fn content_type(&self) -> Option<T> { None }
    fn set_content_type(&mut self, ty: T);
    fn path(&self) -> &ContentPath;
    fn size(&self) -> u64;
    fn read(&mut self, offset: u64, count: u32) -> Option<&[u8]>;
}
```

Five methods, on purpose. The scanner, the filter, identifiers, analyzers, and extractors all work from this surface.

| Method                | Role                                                                                                                 |
| --------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `content_type()`      | `Some(ty)` pins the type and **skips identifiers**. `None` (the default) means the scanner must identify the object. |
| `set_content_type(ty)` | The scanner calls this on [`ReprocessAsType`](analyzer.md#analysisoutcome). Built-in contents store `ty` so the next analysis pass skips identifiers. |
| `path()`              | Name used by the filter, by name/extension identification, and interned into the result tree.                        |
| `size()`              | Total byte length. Directories report `0`.                                                                           |
| `read(offset, count)` | Random-access window into the bytes. Returns a slice borrowed from `self`.                                           |

Analyzers and extractors receive `&mut dyn Content<T>` so they can pull just the windows they need. Nothing in the trait requires the whole payload to be in memory. Typed helpers (`read_exact`, integers, `read_struct`) live on `ContentReadExt`, not on the trait.

## `read`

`read` is random access. Calling it with offset `1000` and then offset `0` is fine; there is no hidden cursor on `Content` itself.

The contract:

- `Some(&[])` — `offset` is exactly at the end (`offset == size()`).
- `None` — `offset` is past the end, or the source cannot serve the request (unreadable file, I/O error after a failed open, a folder).
- A **short** slice — fewer bytes than `count` — is not end of file. It may be EOF, or it may be an implementation boundary (one cache page of a `FileContent`). Advance by the slice length and read again.

That last point matters for anything that streams. [`ContentReader`](extractor.md) (used by the ZIP extractor) already treats a short `Content::read` as “copy what you got and continue,” and `None` before `size()` as `UnexpectedEof`.

## `ContentReadExt`

`read` is the primitive. Plugins usually want a contiguous window, a copy into their own buffer, or a typed integer. `ContentReadExt` is a blanket extension trait: it is implemented for every `Content<T>`, including `&mut dyn Content<T>`. Bring it into scope (`use content_scan::ContentReadExt`, or `use content_scan::*`) and the methods below are available. You do not implement it yourself.

### `read_exact`

`read_exact(offset, count)` is `read` plus a length check. It returns `Some` only if **that single call** produced exactly `count` bytes. `count == 0` is always `Some(&[])`.

`None` does not always mean the bytes are missing. A paged source — `FileContent` with shared read access, or any custom `read` that stops at a cache page — may return a short slice at a window boundary. `read_exact` then fails even though a later `read` at the next page would succeed. Use `read_into` when the range might cross a window.

### `read_into`

`read_into(offset, count, output_buf)` copies into a caller buffer, looping across short reads until the target is reached or the content ends. The number of bytes copied is the minimum of `count`, remaining content (`size - offset`), and `output_buf.len()`. The call never reads past the end of the content and never writes past the end of the buffer.

- `Some(n)` — `output_buf[..n]` is filled. `n` may be less than `count` at EOF or if the buffer is smaller.
- `Some(0)` — `offset` is exactly at the end, `count` is `0`, or the buffer is empty. Compare `offset` to `size()` if you need to tell those apart.
- `None` — `offset` is past the end, or an underlying `read` failed **at any point**. Partial copies are discarded; only `Some(n)` reports progress.

### Bytes and integers

These all go through `read_exact`, so they inherit the single-window rule: `None` if the content ends (or errors) before the value is available, **or** if the value sits across a page boundary.

| Method                        | Width | Endianness   |
| ----------------------------- | ----- | ------------ |
| `read_byte`                   | 1     | —            |
| `read_be_u16` / `read_le_u16` | 2     | big / little |
| `read_be_u32` / `read_le_u32` | 4     | big / little |
| `read_be_u64` / `read_le_u64` | 8     | big / little |
| `read_be_i16` / `read_le_i16` | 2     | big / little |
| `read_be_i32` / `read_le_i32` | 4     | big / little |
| `read_be_i64` / `read_le_i64` | 8     | big / little |

```rust
let Some(soi) = content.read_be_u16(0) else {
    return AnalysisOutcome::Continue;
};
if soi != 0xFFD8 {
    return AnalysisOutcome::Continue;
}
let Some(height) = content.read_be_u16(offset + 5) else {
    return AnalysisOutcome::Continue;
};
```

### `read_struct`

`read_struct::<S>(offset)` copies `size_of::<S>()` bytes into an `S`. It uses `read_into`, so it **does** cross page boundaries, and `offset` has no alignment requirement.

- `Some(s)` if that many bytes are available starting at `offset`.
- `None` if the read would run past the end, if `read` fails, or if `size_of::<S>()` exceeds `u32::MAX`.
- A zero-sized `S` returns `Some` without touching the content.

This is `unsafe`. Every bit pattern of length `size_of::<S>()` must be a valid `S`. That holds for `#[repr(C)]` plain-data structs of integers, floats, and arrays of those. It is undefined behavior for a type with validity invariants (`bool`, `char`, references, `NonZero*`, or an enum with a restricted set of discriminants) — arbitrary file bytes may not be a valid instance.

```rust
#[repr(C)]
#[derive(Copy, Clone)]
struct CoffHeader {
    machine: u16,
    number_of_sections: u16,
    time_date_stamp: u32,
}

let Some(hdr) = unsafe { content.read_struct::<CoffHeader>(offset) } else {
    return AnalysisOutcome::Continue;
};
```

## Ready-made implementations

Three types ship with the crate. You pick one for the root of a scan; extractors often return the same types for children.

### `BufferContent<T>`

An owned `Vec<u8>` plus a **synthetic** UTF-8 path (`ContentPath::from_str`). This is what [Your first scan](../chapter-1/first_scan.md) used, and what extractors typically emit for small nested items.

```rust
BufferContent::<MyType>::new(b"TXBF hello", "test.txt");
BufferContent::<MyType>::from_vec(owned, "test.txt");          // move, no copy
BufferContent::<MyType>::with_content_type(buf, "test.txt", MyType::Text);
BufferContent::<MyType>::from_parts(vec, path_string, Some(MyType::Text));
```

`new` / `from_vec` leave the type unset, so identifiers run. `with_content_type` / `from_parts(..., Some(ty))` pin the type and skip identification.

### `FileContent<T>`

A file on disk. The path goes through `ContentPath::from_os`, so a non-UTF-8 filesystem name stays openable. The file is **opened lazily** on the first `read()`; constructing a `FileContent` you never read costs only the path (and, for `new` / `with_content_type`, one `stat` for the size). A file that cannot be opened behaves as empty: `size()` is `0` and every `read` returns `None`.

```rust
FileContent::<MyType>::new("photo.png", false);                    // shared read
FileContent::<MyType>::with_content_type("photo.png", MyType::Png, false);
FileContent::<MyType>::with_size("photo.png", 4096, false);         // size already known
```

`exclusive` chooses how the file is opened:

- `false` — shared read access and an LRU page cache. Use this when the file might already be open elsewhere.
- `true` — exclusive lock and a memory map. Files already open in another process will fail to open.

`with_size` skips the `stat` at construction. `FolderExtractor` uses that when the directory entry’s metadata already has the length.

### `FolderContent<T>`

A directory marker, not a byte stream: `size()` is `0` and `read()` always returns `None`. There is nothing to identify a folder by, so the type is **mandatory**:

```rust
let mut root = FolderContent::<MyType>::with_content_type("./src", MyType::Folder);
let result = scanner.scan(&mut root, false);
```

The scanner sees `content_type() == Some(Folder)` and dispatches straight to the extractor registered for that variant. Pass `filter_root = false` when the filter is written for files (extensions, names) and would reject the directory itself. Details are in [Folder](../chapter-5/folder.md) and [Recursion and filter_root](../chapter-3/recursion.md).

## Choosing a root

| You have                                                  | Wrap it in                                             |
| --------------------------------------------------------- | ------------------------------------------------------ |
| Bytes already in memory                                   | `BufferContent`                                        |
| A file path                                               | `FileContent`                                          |
| A directory to walk                                       | `FolderContent` plus a `FolderExtractor` for that type |
| Something else (mmap region, archive entry, network blob) | implement `Content<T>`                                 |

A child produced by an extractor is just another `Content`. A ZIP member might become a `BufferContent` (small) or a `FileContent` on a temp file (large). The scanner does not care.

## Pinning a type versus identifying

`content_type()` is the fork in the [pipeline](architecture.md):

- `Some(ty)` — identifiers do not run. Analyzers and extractors for `ty` do.
- `None` — the identifier table runs (magic, name, extension, then custom `validate`). The object may remain unidentified; generic analyzers still run.

Use a pinned type when you already know (a folder, a buffer you just decoded, a child you constructed as `Number`). Leave it unset when the bytes or the name should decide.

## Implementing `Content` yourself

You need a `ContentPath`, a size, and `read`. Typical reasons:

- A region of a parent you do not want to copy.
- A format the built-ins do not cover.
- A test double.

Keep `read` honest about short slices and `None`. `ContentReadExt` is implemented automatically for your type: `read_exact` and the integer helpers will fail at those short-slice boundaries, while `read_into` / `read_struct` will loop. If a stream-oriented library must consume the object, wrap an [`OwnedContentPtr`](extractor.md) in `ContentReader` rather than changing the trait to `std::io::Read`. Those two types exist for extractors; they are not part of `Content` itself.

The next two pages are the type parameter and the name. Everything later in the book assumes both.
