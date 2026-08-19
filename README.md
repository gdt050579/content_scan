# content_scan

A small, extensible **content scanning framework** for Rust.

`content_scan` gives you a plug-in style pipeline for **identifying**, **analyzing** and **extracting** structured content out of arbitrary byte streams (files, buffers, embedded archives, etc.). You define your own content types and plug in identifiers, analyzers and extractors — the scanner takes care of dispatching, recursion and filtering.

Typical use cases:

- File-type / MIME-like detection based on magic bytes, extensions or file names.
- Static analysis pipelines (metrics, heuristics, feature extraction).
- Recursive scanning of container formats (archives, bundles, embedded blobs) with a configurable depth limit.
- Walking a directory tree and scanning every file in it (`FolderContent` + `FolderExtractor`).
- Building custom "scanners" (antivirus-like tools, indexers, linters, forensics tools, …) on top of a common core.

> Status: early / experimental (`0.1.x`).

---

## Table of contents

- [content\_scan](#content_scan)
  - [Table of contents](#table-of-contents)
  - [Workspace layout](#workspace-layout)
  - [Core concepts](#core-concepts)
  - [Getting started](#getting-started)
  - [Examples](#examples)
    - [Counting vowels](#counting-vowels)
    - [Summing numbers extracted from text](#summing-numbers-extracted-from-text)
    - [Reading PNG / BMP / JPEG dimensions](#reading-png--bmp--jpeg-dimensions)
    - [Finding and decoding Base64](#finding-and-decoding-base64)
    - [Computing MD5 hashes](#computing-md5-hashes)
    - [Listing ZIP files](#listing-zip-files)
  - [API overview](#api-overview)
    - [`ContentType`](#contenttype)
    - [`Content`](#content)
    - [`ContentPath`](#contentpath)
    - [`ContentIdentifier`](#contentidentifier)
    - [`ZipIdentifier`](#zipidentifier)
    - [`ContentAnalyzer`](#contentanalyzer)
    - [`ContentExtractor`](#contentextractor)
    - [`ExtractionSession`](#extractionsession)
    - [`OwnedContentPtr`](#ownedcontentptr)
    - [`ExtractionContext`](#extractioncontext)
    - [Requesting extraction](#requesting-extraction)
    - [Walking the file system](#walking-the-file-system)
    - [`Filter` / `FilterBuilder`](#filter--filterbuilder)
    - [`Scanner` / `ScannerBuilder`](#scanner--scannerbuilder)
  - [`Context` / `ScanResult`](#context--scanresult)
  - [Navigating the scan result tree](#navigating-the-scan-result-tree)
- [Scanning pipeline](#scanning-pipeline)
  - [Building \& testing](#building--testing)
  - [License](#license)

---

## Workspace layout

This repository is a Cargo workspace with three members:

| Crate                     | Path                                                  | Purpose                                                                                        |
| ------------------------- | ----------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `content_scan`            | [`content_scan/`](content_scan)                       | The main library: scanner, traits, matchers, filters.                                          |
| `content_scan_proc_macro` | [`content-scan-proc-macro/`](content-scan-proc-macro) | Companion proc-macro crate exposing `#[derive(ContentType)]`. Re-exported from `content_scan`. |
| `examples`                | [`examples/`](examples)                               | Runnable examples (`sum`, `vowals`, `image_size`, `base64_find`, `md5`, `find_zip`).              |

You normally only depend on `content_scan` — the proc-macro is re-exported for you.

---

## Core concepts

The framework is built around a few small traits:

- **`ContentType`** — a `#[repr(u16)]` enum describing the kinds of content your scanner knows about. Derived with `#[derive(ContentType)]`.
- **`Content<T>`** — an abstract, seekable, read-only byte source with a `ContentPath` and a size. Ready-made `BufferContent<T>` (in-memory), `FileContent<T>` (memory-mapped file) and `FolderContent<T>` (a directory, used as a container) implementations are provided.
- **`ContentPath`** — the path or synthetic address of a piece of content. Holds a UTF-8 printable view always, and keeps the original OS path when the name is not valid UTF-8 so the file can still be opened.
- **`ContentIdentifier<T>`** — decides *what* a piece of content is (by magic bytes, extension, or file name) and validates the guess.
- **`ContentAnalyzer<T>`** — reads content and produces information (stored in a shared `Context`). Analyzers can also queue extra extraction passes with `context.request_extract(ty)`.
- **`ContentExtractor<T>`** — pulls sub-contents out of a container. `create_session` receives an `OwnedContentPtr` to the parent and an `ExtractionContext` describing the region to look at (`offset`, optional `length`, optional `params`), and returns an `ExtractionSession`. The session then yields children via `advance` / `extract`. Per-session state (cursors, open archives, the current `Entry`) lives on that session object, not on the extractor — one extractor instance is shared and sessions may nest. `FolderExtractor<T>` is a ready-made extractor that enumerates a directory.
- **`Filter`** — decides which paths / sizes should be processed at all.
- **`Scanner<T>`** — the orchestrator; built via `ScannerBuilder<T>`.
- **`ScanResult<T>` / `ScanContentHandle`** — after a scan, the framework exposes the full **tree** of visited objects (parent / child / sibling links), each with its interned path, resolved content type and its own local `VarMap`.

Analyzers are either **specific** to a `ContentType` or **generic** (run on every scanned object), and each is registered with a `priority` byte to control execution order. Extractors are registered per type and run in registration order — both when the current object is that type, and when an analyzer [requests](#requesting-extraction) that type.

---

## Getting started

Add the crate to your `Cargo.toml` (from a local path or git checkout while the crate is not yet published):

```toml
[dependencies]
content_scan = { path = "path/to/content_scan/content_scan" }
```

Bring the prelude into scope:

```rust
use content_scan::*;
```

Define your content types, plug in identifiers / analyzers / extractors, build a `Scanner`, and call `scan()`:

```rust
use content_scan::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ContentType)]
#[repr(u16)]
enum MyType {
    TextBuffer,
}

// ... implement ContentIdentifier / ContentAnalyzer / ContentExtractor ...

fn main() {
    let mut scanner = ScannerBuilder::<MyType>::new()
        // .add_identifier(MyType::TextBuffer, MyIdentifier {})
        // .add_analyzer(MyType::TextBuffer, 0, MyAnalyzer {})
        // .add_extractor(MyType::TextBuffer, MyExtractor::default())
        .build();

    let mut content = BufferContent::<MyType>::new(b"...", "input.bin");
    // the second argument decides whether the Filter is applied to the root object itself
    let result = scanner.scan(&mut content, true);

    println!("scanned {} objects", result.objects_scanned());
}
```

---

## Examples

The [`examples/`](examples) directory contains runnable programs. From the workspace root:

```bash
cargo run --example vowals
cargo run --example sum
cargo run --example image_size -- path/to/image.png
cargo run --example image_size -- path/to/folder
cargo run --example base64_find
cargo run --example base64_find -- path/to/file_or_folder
cargo run --example md5 -- path/to/file_or_folder
cargo run --example find_zip -- path/to/folder
```

### Counting vowels

A single analyzer that counts vowels in a magic-tagged buffer. See [`examples/vowals/main.rs`](examples/vowals/main.rs).

```rust
use content_scan::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ContentType)]
#[repr(u16)]
enum MyType {
    TextBuffer,
}

struct VowelAnalyzer;
impl ContentAnalyzer<MyType> for VowelAnalyzer {
    fn analyze(&mut self, content: &mut dyn Content<MyType>, context: &mut Context) -> NextAction {
        let sz = content.size();
        let mut count = 0u32;
        for i in 4..sz {
            if let Some(b) = content.read(i, 1) {
                let b = b[0].to_ascii_lowercase();
                if matches!(b, b'a' | b'e' | b'i' | b'o' | b'u') {
                    count += 1;
                }
            }
        }
        context.global().set(var!("count_vowels"), count);
        NextAction::Continue
    }
}

struct TextBufferIdentifier;
impl ContentIdentifier<MyType> for TextBufferIdentifier {
    fn identify_method(&self) -> Option<IdentifyMethod> {
        Some(IdentifyMethod::Magic(b"TXBF"))
    }
    fn validate(&self, _: &mut dyn Content<MyType>) -> bool { true }
}

fn main() {
    let mut scanner = ScannerBuilder::new()
        .add_analyzer(MyType::TextBuffer, 0, VowelAnalyzer {})
        .add_identifier(MyType::TextBuffer, TextBufferIdentifier {})
        .build();

    let mut b = BufferContent::<MyType>::with_content_type(
        b"TXBF   Hellow World !", "test.txt", MyType::TextBuffer,
    );
    let res = scanner.scan(&mut b, true);
    println!("count_vowels: {}", res.global().get::<u32>(var!("count_vowels")).unwrap());
}
```

### Summing numbers extracted from text

This example shows the full pipeline — a text container is *identified* by magic bytes, an *extractor* pulls numeric substrings out of it as independent `Number` contents, and a numeric *analyzer* both sums them into a shared **global** variable and stashes each value into a per-object **local** `VarMap`. After the scan, the example walks the resulting tree via `ScanResult`. The extractor's cursor lives on an [`ExtractionSession`](#extractionsession) that holds an [`OwnedContentPtr`](#ownedcontentptr) to the parent, so a nested extraction can open another session without overwriting the first. See [`examples/sum/main.rs`](examples/sum/main.rs).

```rust
struct NumericAnalyzer;
impl ContentAnalyzer<MyTypes> for NumericAnalyzer {
    fn analyze(&mut self, content: &mut dyn Content<MyTypes>, context: &mut Context) -> NextAction {
        let value = u32::from_str_radix(
            std::str::from_utf8(content.read(0, content.size() as u32).unwrap()).unwrap(),
            10,
        ).unwrap();

        // aggregate into the global VarMap...
        if !context.global().update(var!("sum"), |v: &mut u32| *v += value) {
            context.global().set(var!("sum"), value);
        }
        // ...and remember this object's own value in its local VarMap
        context.local().set(var!("value"), value);
        NextAction::Continue
    }
}

fn main() {
    let mut scanner = ScannerBuilder::new()
        .add_analyzer(MyTypes::Number, 0, NumericAnalyzer {})
        .add_extractor(MyTypes::Text, NumericExtractor::default())
        .add_identifier(MyTypes::Text, TextIdentifier {})
        .build();

    let mut b = BufferContent::<MyTypes>::new(b"TXT   1+2+3=", "test.txt");
    let res = scanner.scan(&mut b, true);
    println!("sum: {}", res.global().get::<u32>(var!("sum")).unwrap_or(0)); // -> 6

    // Walk the scan tree: root ("test.txt") -> children ("number", "number", "number")
    let root = res.root().unwrap();
    println!("root: {}", res.path(root).unwrap());
    let mut c = res.child(root).unwrap();
    loop {
        let v = res.local(c).unwrap().get::<u32>(var!("value")).unwrap_or(0);
        println!("- child: {} => {}", res.path(c).unwrap(), v);
        match res.next_sibling(c) {
            Some(next) => c = next,
            None => break,
        }
    }
}
```

### Reading PNG / BMP / JPEG dimensions

The [`image_size`](examples/image_size) example registers one identifier + analyzer pair per image format (`png.rs`, `bmp.rs`, `jpeg.rs`) and each analyzer stores a `Size { width, height }` in the object's **local** `VarMap`. It accepts either a single file or a whole directory:

```bash
cargo run --example image_size -- photo.jpg
cargo run --example image_size -- ./pictures
```

A file is wrapped in a `FileContent` and scanned directly. A directory is wrapped in a `FolderContent` tagged with the user's own `ImageType::Folder` variant, and a recursive `FolderExtractor` registered for that type turns each directory entry (including nested folders) into a child content object:

```rust
let mut scanner = ScannerBuilder::new()
    .filter(
        FilterBuilder::new()
            .include_extensions(Precedence::Medium, &["jpg", "jpeg", "bmp", "png"])
            .deny_the_rest()
            .build(),
    )
    .add_identifier(ImageType::Png, png::PngIdentifier {})
    .add_analyzer(ImageType::Png, 0, png::PngAnalyzer {})
    // ...bmp / jpeg...
    .add_extractor(ImageType::Folder, FolderExtractor::<ImageType>::new(true, false))
    .build();

let res = if Path::new(&path).is_dir() {
    let mut content = FolderContent::<ImageType>::with_content_type(&path, ImageType::Folder);
    scanner.scan(&mut content, false)  // don't filter the root: a folder has no image extension
} else {
    let mut content = FileContent::<ImageType>::new(&path, false);
    scanner.scan(&mut content, true)
};
```

Note the `false` passed to `scan` for the directory case: the filter only allows image extensions, so applying it to the root folder would reject the scan before it starts. The example then prints the resulting tree, indented by depth, with `{width} x {height}` next to every recognized image.

### Finding and decoding Base64

[`base64_find`](examples/base64_find/main.rs) shows an analyzer **requesting** extractors of another type. A `Text` analyzer locates contiguous Base64 runs, emits `request_extract(MyTypes::Base64)` for each hit, a `Base64` extractor decodes the slice into a `Vec<u8>` tagged `Base64Decoded`, and a `Base64Decoded` analyzer prints that buffer. With no arguments it scans a built-in sample; pass a file or folder to scan real content:

```bash
cargo run --example base64_find
cargo run --example base64_find -- notes.txt
cargo run --example base64_find -- ./inbox
```

```rust
impl ContentAnalyzer<MyTypes> for Base64Finder {
    fn analyze(&mut self, content: &mut dyn Content<MyTypes>, context: &mut Context<MyTypes>) -> NextAction {
        // ...locate a run at `start` of length `len`...
        context.request_extract(MyTypes::Base64).at(start).len(len).emit();
        NextAction::Continue
    }
}

impl ContentExtractor<MyTypes> for Base64Extractor {
    fn create_session(
        &mut self,
        content: OwnedContentPtr<MyTypes>,
        extract_context: &ExtractionContext,
    ) -> Option<Box<dyn ExtractionSession<MyTypes>>> {
        let length = extract_context.length.unwrap_or(content.size().saturating_sub(extract_context.offset));
        if length == 0 {
            return None;
        }
        Some(Box::new(Base64Session {
            content,
            offset: extract_context.offset,
            length,
            done: false,
            entry: Entry::default(),
        }))
    }
}

impl ExtractionSession<MyTypes> for Base64Session {
    fn extract(&mut self) -> Option<Box<dyn Content<MyTypes>>> {
        let encoded = read_range(&mut *self.content, self.offset, self.length)?;
        let decoded = decode_base64(&encoded)?;
        Some(Box::new(BufferContent::<MyTypes>::from_parts(
            decoded,
            format!("base64@{}", self.offset),
            Some(MyTypes::Base64Decoded),
        )))
    }
    // ...
}

impl ContentAnalyzer<MyTypes> for Base64DecodedAnalyzer {
    fn analyze(&mut self, content: &mut dyn Content<MyTypes>, _: &mut Context<MyTypes>) -> NextAction {
        let buf = content.read(0, content.size() as u32).unwrap_or(&[]);
        println!("{}: {}", content.path().as_printable_string(), String::from_utf8_lossy(buf));
        NextAction::Continue
    }
}
```

The extractor is registered for `Base64`, not for `Text`. It only runs because the finder queued that type; the parent file itself is never identified as `Base64`.

### Computing MD5 hashes

[`md5`](examples/md5/main.rs) hashes every file with a **generic** analyzer. `ComputeHashAnalyzer` is registered with `add_generic_analyzer`, so it runs on every scanned object regardless of type. A recursive `FolderExtractor` walks the tree when the argument is a directory; folders themselves are skipped (they have no bytes to hash).

```bash
cargo run --example md5 -- README.md
cargo run --example md5 -- ./content_scan/src
```

```rust
struct ComputeHashAnalyzer;
impl ContentAnalyzer<MyTypes> for ComputeHashAnalyzer {
    fn analyze(&mut self, content: &mut dyn Content<MyTypes>, _: &mut Context<MyTypes>) -> NextAction {
        if content.content_type() == Some(MyTypes::Folder) {
            return NextAction::Continue;
        }
        // read the file in chunks, feed Md5, print "{hex}  {path}"
        NextAction::Continue
    }
}

fn main() {
    let mut scanner = ScannerBuilder::new()
        .add_generic_analyzer(0, ComputeHashAnalyzer {})
        .add_extractor(MyTypes::Folder, FolderExtractor::<MyTypes>::new(true, false))
        .build();
    // FileContent for a file, FolderContent + Folder extractor for a directory
}
```

### Listing ZIP files

[`find_zip`](examples/find_zip/main.rs) walks a directory with `FolderExtractor` and uses the built-in [`ZipIdentifier`](content_scan/src/implementations/zip.rs) (local-file magic `PK\x03\x04`, then an EOCD check in `validate`) to decide which files are ZIPs. A type-specific analyzer prints each match. Identification is by content, not by `.zip` extension.

```bash
cargo run --example find_zip -- ./archives
```

```rust
let mut scanner = ScannerBuilder::new()
    .add_identifier(MyTypes::Zip, ZipIdentifier::new())
    .add_analyzer(MyTypes::Zip, 0, ZipPrinter {})
    .add_extractor(MyTypes::Folder, FolderExtractor::<MyTypes>::new(true, false))
    .build();
```

---

## API overview

### `ContentType`

Every scanner is parameterized by a user-defined enum implementing `ContentType`. The `#[derive(ContentType)]` macro implements the trait automatically. Requirements:

- The enum must be annotated with `#[repr(u16)]`.
- Variants must be **unit variants** with **no explicit discriminants**.
- Derive `Ord` and `PartialOrd` as well — the trait requires them.
- Up to `65536` variants are supported.

```rust
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ContentType)]
#[repr(u16)]
enum MyTypes {
    Number,
    Text,
}
```

`bool` also implements `ContentType` and is used internally by the matcher / filter machinery.

### `Content`

```rust
pub trait Content<T: ContentType> {
    fn content_type(&self) -> Option<T> { None }
    fn path(&self) -> &ContentPath;
    fn size(&self) -> u64;
    fn read(&mut self, offset: u64, count: u32) -> Option<&[u8]>;
}
```

A `Content` is any addressable byte source. It exposes a [`ContentPath`](#contentpath) (used for identification, filtering, and display), a `size`, and a `read(offset, count)` method that returns a borrowed slice.

Three implementations ship with the crate — an in-memory one, a file-backed one, and a directory marker:

```rust
BufferContent::<MyType>::new(buffer, "path.ext");
BufferContent::<MyType>::with_content_type(buffer, "path.ext", MyType::Text);
BufferContent::<MyType>::from_parts(vec, "path".into(), Some(MyType::Text));

FileContent::<MyType>::new("path/to/file.bin", false);           // impl AsRef<Path>; false = shared read
FileContent::<MyType>::with_content_type("path/to/file.bin", MyType::Text, false);
FileContent::<MyType>::with_size("path/to/file.bin", 4096, false);   // size already known, skip the stat

FolderContent::<MyType>::with_content_type("path/to/dir", MyType::Folder);
```

`BufferContent` constructors take a `&str` and treat it as a **synthetic** address (always UTF-8). `FileContent` and `FolderContent` take `impl AsRef<Path>` and go through `ContentPath::from_os`, so a real filesystem name that is not valid UTF-8 stays openable.

`FileContent` opens the file lazily on the first `read()`. Pass `exclusive = true` to memory-map with an exclusive lock (files already open elsewhere will fail). Pass `false` for shared access through an LRU page cache. `with_size` is useful when the size is already known (a directory walk has just stat'ed the entry, for instance) and you want to avoid a second filesystem call.

`FolderContent` represents a directory rather than bytes: its `size()` is `0` and `read()` always returns `None`. It always reports the content type you give it, so the scanner dispatches straight to the extractor registered for that type — see [Walking the file system](#walking-the-file-system).

You can also implement `Content<T>` for memory-mapped regions, network streams, archive entries, etc.

### `ContentPath`

`ContentPath` is the path type used everywhere a piece of content is named: `Content::path()`, `Entry::path`, filter callbacks, and (as interned bytes) the scan result tree. It covers both **virtual addresses** (`archive.zip://inner/file.txt`, `"number"`) and **real OS paths**, including names that are not valid UTF-8.

```rust
pub struct ContentPath { /* ... */ }

impl ContentPath {
    pub fn from_str(s: &str) -> Self;            // synthetic / known UTF-8
    pub fn from_os(p: &Path) -> Self;            // real filesystem path
    pub fn empty() -> Self;                      // reusable empty buffer
    pub fn clear(&mut self);                     // reset, keep capacity
    pub fn set_from_str(&mut self, s: &str);     // reuse allocation
    pub fn set_from_os(&mut self, p: &Path);     // reuse allocation
    pub fn as_printable_string(&self) -> &str;   // always valid UTF-8 (lossy if needed)
    pub fn as_path(&self) -> &Path;              // openable OS path
    pub fn as_bytes(&self) -> &[u8];             // filtering / identification
    pub fn is_lossless(&self) -> bool;
}
```

- **`from_str` / `set_from_str`** — use for archive members, in-memory buffers, and any label you already know is UTF-8. Do **not** stringify a real OS path and pass it here: a non-UTF-8 filesystem name would lose the bytes needed to reopen it.
- **`from_os` / `set_from_os`** — use for `DirEntry`, `PathBuf`, and anything that came from the filesystem. When the path is valid UTF-8 only the string is stored. Otherwise the original `OsString` is kept alongside a lossy printable view (`U+FFFD` for invalid sequences), so `as_path()` still names the original file and `is_lossless()` is `false`.
- **`empty` / `clear`** — construct or reset an empty path while keeping the string allocation. Sessions typically `clear` then `set_from_os` / `set_from_str` on a reused `Entry`.
- **`as_printable_string`** — always available, never fails. Safe to log or print. Exact only when `is_lossless()` is `true`.
- **`as_path`** — the `&Path` to hand to `fs::read_dir`, `File::open`, and similar. For a non-UTF-8 path this is the preserved OS name, not the lossy string.
- **`as_bytes`** — what identification and `Filter` inspect (file name / extension). On Unix this is the faithful path bytes; on Windows it is the printable string's bytes.

`ContentPath` implements `From` for `&str`, `String`, `&String`, `&Path`, `PathBuf`, and `&PathBuf`. UTF-8 strings go through `from_str` (or `with_string` when already owned); OS paths go through `from_os`. `FileContent` and `FolderContent` take `impl AsRef<Path>`, so `String` and `PathBuf` work at those constructors too.

`FolderExtractor` fills each `Entry` with `set_from_os`, so a directory walk does not drop non-UTF-8 names the way a `to_str().unwrap_or_default()` conversion would.

### `ContentIdentifier`

Identifiers tell the scanner *what type* a piece of content is when the content itself doesn't already report one.

```rust
pub trait ContentIdentifier<T: ContentType> {
    fn identify_method(&self) -> Option<IdentifyMethod>;
    fn validate(&self, content: &mut dyn Content<T>) -> bool;
}

pub enum IdentifyMethod {
    Magic(&'static [u8]),
    MultipleMagic(&'static [&'static [u8]]),
    Extension(&'static str),
    Extensions(&'static [&'static str]),
    Name(&'static str),
    Names(&'static [&'static str]),
}
```

Fast identification is performed with an internal matcher (single-pattern, packed magic table, or trie depending on the number and shape of patterns). After a fast match, `validate()` is called to confirm the guess. Overlapping magic prefixes resolve to the **longest** match in both the packed table and the trie.

Extension and file-name methods are ASCII case-insensitive: both the registered pattern and the path basename / extension are compared in lowercase. `Notes.TXT` matches `Extension("txt")`; `makefile` matches `Name("Makefile")`. Magic methods remain exact byte matches. A magic sequence (`Magic` / `MultipleMagic`) must be at most **16 bytes** — that is the window the scanner reads (`content.read(0, 16)`). `ScannerBuilder::build` panics if a registered magic is longer. To inspect bytes past that window, return `None` from `identify_method` and call `content.read` in `validate`.

If `identify_method` returns `None`, `validate()` is still called — after magic, file name, and extension have all been considered — so you can classify content with custom logic (payload bytes via `content.read`, path shape, size, …). Those identifiers are tried in the order they were registered.

At most **one identifier per `ContentType`** may be registered; the builder will panic otherwise. It also panics if a `Magic` / `MultipleMagic` pattern is longer than 16 bytes.

### `ZipIdentifier`

`ZipIdentifier<T>` is a built-in identifier for ZIP archives. Fast identification is the local-file magic `PK\x03\x04`; `validate` then looks for an End of Central Directory record in the tail of the content, so a file that merely starts with those four bytes is rejected. Identification is by content, not by `.zip` extension.

```rust
.add_identifier(MyTypes::Zip, ZipIdentifier::new())
```

### `ContentAnalyzer`

```rust
pub trait ContentAnalyzer<T: ContentType> {
    fn analyze(&mut self, content: &mut dyn Content<T>, context: &mut Context<T>) -> NextAction;
}
```

Analyzers inspect content and write results into the shared `Context`. Use `context.local()` for per-object findings and `context.global()` for scan-wide aggregates. To pull nested content out of the current object using extractors registered for a **different** type — for example an analyzer that locates an embedded ZIP and wants the Zip extractor to open it — call `context.request_extract(ty)` and [emit an extraction request](#requesting-extraction). Only analyzers return `NextAction`; that value controls the rest of **this** object:

- `NextAction::Continue` — run the next analyzer for this object; after the last analyzer, run extractors.
- `NextAction::Skip` — stop this object: do not run remaining analyzers or any extractors on it. Siblings and later objects still scan.
- `NextAction::Exit` — abort the entire scan.

Register analyzers with:

- `add_analyzer(content_type, priority, analyzer)` — runs only when the content matches `content_type`.
- `add_generic_analyzer(priority, analyzer)` — runs for every scanned object, regardless of type.

Within a bucket, analyzers execute in ascending `priority` order.

### `ContentExtractor`

```rust
pub trait ContentExtractor<T: ContentType> {
    fn create_session(
        &mut self,
        content: OwnedContentPtr<T>,
        extract_context: &ExtractionContext,
    ) -> Option<Box<dyn ExtractionSession<T>>>;
}

pub struct Entry {
    pub path: ContentPath,
    pub size: u64,
    pub skip_from_filtering: bool,
}
```

Extractors turn a container into a stream of children. The scanner calls `create_session` once per parent; the returned [`ExtractionSession`](#extractionsession) then enumerates children. Methods return `Option` — they do **not** return `NextAction` and cannot Skip or Exit on their own.

`create_session` is called after that object's analyzers have run. It receives:

- an [`OwnedContentPtr`](#ownedcontentptr) to the parent — store it on the session if `advance` / `extract` need to read the parent;
- an [`ExtractionContext`](#extractioncontext) describing the region to look at (`offset`, optional `length`, optional `params`). Copy those fields into the session; the context is only valid for this call.

Return `Some(session)` to start enumerating, or `None` to skip this extractor (the scanner moves on to the next one registered for the same type). The session is dropped when enumeration ends, when a nested child's analyzer returns `NextAction::Exit`, or when the scanner moves on. Implement `Drop` on the session if you need to close files or free buffers.

An extractor registered for type `T` runs in two situations:

- The current object was **identified as `T`**. The context then covers the whole object (`offset = 0`, `length = Some(content.size())`, `params = None`).
- An analyzer **requested** extraction of `T` from the current object via [`context.request_extract`](#requesting-extraction). The context then carries the requested offset, length, and params. The parent does not need to have been identified as `T`.

One extractor instance is shared by every object of its type, and `create_session` can be re-entered while a previous session is still live (nested containers). Keep configuration on the extractor and put cursors, open archives, and the current `Entry` on the session. Extractors are registered with `add_extractor` for a specific `ContentType`. Multiple extractors for the same type run in registration order.

### `ExtractionSession`

```rust
pub trait ExtractionSession<T: ContentType> {
    fn advance(&mut self) -> Option<&Entry>;
    fn extract(&mut self) -> Option<Box<dyn Content<T>>>;
}
```

A session is a live extraction of children from one parent. The scanner drives it as a short loop:

1. `advance` — move to the next child and return a lightweight `Entry` describing its path/size. Returning `None` ends the stream and the session is dropped.
2. `extract` — materialize that child as a boxed `Content<T>`. The scanner then recursively scans it (subject to `max_depth`). Returning `None` skips just this entry; enumeration continues with the next `advance`.

A child's `Skip` does not end this session — the scanner continues with the next `advance`. A child's `Exit` drops the session immediately as the scan unwinds.

Set `Entry::skip_from_filtering` to `true` to exempt an entry from the active `Filter`. This matters for container entries that would never pass the filter themselves: a `FolderExtractor` restricted to `*.png` files, for example, still has to descend into subdirectories, whose names carry no `.png` extension. Keep one `Entry` as a field on the session and overwrite `entry.path` in place with `ContentPath::set_from_str` (synthetic names) or `ContentPath::set_from_os` (real OS paths) so `advance` does not allocate a new path for every child.

### `OwnedContentPtr`

`OwnedContentPtr<T>` is the handle `create_session` receives instead of a `&mut dyn Content<T>`. It does **not** own the parent — it is a non-owning pointer that [derefs](https://doc.rust-lang.org/std/ops/Deref.html) to `dyn Content<T>`, so a session can call `path()`, `size()`, `read()`, and `content_type()` through it across `advance` / `extract` without fighting the borrow checker.

The scanner guarantees the parent outlives the session. Do not store the pointer after the session is dropped.

A typical extractor therefore looks like this (the `sum` example, simplified):

```rust
struct NumericExtractor;

struct NumericSession {
    content: OwnedContentPtr<MyTypes>,
    pos: u64,
    start: u64,
    len: u64,
    entry: Entry,
}

impl ContentExtractor<MyTypes> for NumericExtractor {
    fn create_session(
        &mut self,
        content: OwnedContentPtr<MyTypes>,
        _: &ExtractionContext,
    ) -> Option<Box<dyn ExtractionSession<MyTypes>>> {
        Some(Box::new(NumericSession {
            content,
            pos: 0,
            start: u64::MAX,
            len: 0,
            entry: Entry::default(),
        }))
    }
}

impl ExtractionSession<MyTypes> for NumericSession {
    fn advance(&mut self) -> Option<&Entry> {
        // ...scan forward through `self.content`, updating `self.pos` / `self.start` / `self.len`...
        self.entry.path.set_from_str("number");
        self.entry.size = self.len;
        self.entry.skip_from_filtering = false;
        Some(&self.entry)
    }
    fn extract(&mut self) -> Option<Box<dyn Content<MyTypes>>> {
        let buf = self.content.read(self.start, self.len as u32)?;
        Some(Box::new(BufferContent::<MyTypes>::with_content_type(buf, "number", MyTypes::Number)))
    }
}
```

### `ExtractionContext`

`ExtractionContext` is what `create_session` receives. It names a window inside the parent `Content` plus an optional parameter map:

```rust
pub struct ExtractionContext<'a> {
    pub offset: u64,               // start byte within the parent
    pub length: Option<u64>,       // Some(n) = known size; None = extractor decides
    pub params: Option<&'a VarMap>, // analyzer-supplied extras, or None
}
```

- **`offset`** — first byte of the region. Type-specific extraction of the parent itself always starts at `0`.
- **`length`** — `Some(n)` when the caller knows the region is `n` bytes. `None` means the extractor determines the extent itself (parse until the format ends, scan to EOF, …). Type-specific extraction of the parent itself passes `Some(content.size())`.
- **`params`** — extras an analyzer attached with [`.param(...)`](#requesting-extraction) (password, codec, flags, …). `None` when the request had no `.param()` calls, including type-specific extraction of the parent itself. The borrow lasts only for `create_session` — copy values into the session if you need them later.

### Requesting extraction

Analyzers that find nested content of another type queue a pass with `context.request_extract(ty)`. The call returns an `ExtractRequestBuilder`; chain setters and commit with `.emit()`. Dropping the builder without `emit` does nothing.

```rust
impl ContentAnalyzer<MyTypes> for PeAnalyzer {
    fn analyze(&mut self, _: &mut dyn Content<MyTypes>, context: &mut Context<MyTypes>) -> NextAction {
        // Found an embedded ZIP inside this PE; run Zip extractors on that slice.
        context.request_extract(MyTypes::Zip)
            .at(0x1000)                          // offset within the parent
            .len(4096)                           // optional; omit if unknown
            .param(var!("password"), "secret")   // optional; repeatable
            .emit();
        NextAction::Continue
    }
}
```

Builder methods:

| Method | Effect |
| ------ | ------ |
| `at(offset)` | Byte offset within the parent. Defaults to `0`. |
| `len(n)` | Asserts the region is `n` bytes. Omit to leave `length = None`. |
| `param(key, value)` | Adds one extractor-specific parameter. The first call reserves a pooled `VarMap`; later calls write into the same map. A request with no `.param()` carries no map (`params = None`). |
| `emit()` | Commits the request. Required — the builder is `#[must_use]`. |

After this object's analyzers finish, the scanner:

1. Runs extractors registered for the object's **own** identified type (whole object).
2. Then, in emission order, runs extractors registered for each **requested** type on the same parent, with that request's `ExtractionContext`.

The current object does not need to have been identified as the requested type. Several requests (of the same or different types) may be emitted from one analyzer; each is independent. Nested child scans start with an empty request queue.

The Zip extractor then copies the region out of `create_session`:

```rust
fn create_session(
    &mut self,
    content: OwnedContentPtr<MyTypes>,
    ctx: &ExtractionContext,
) -> Option<Box<dyn ExtractionSession<MyTypes>>> {
    let start = ctx.offset;
    let len = ctx.length; // None = parse until the ZIP ends
    let password = ctx.params.and_then(|p| p.get::<&str>(var!("password"))).map(str::to_string);
    Some(Box::new(ZipSession { content, start, len, password, ... }))
}
```

### Walking the file system

`FolderExtractor<T>` is a built-in extractor that enumerates a directory and emits one child per entry. Pair it with a `FolderContent` root and a `ContentType` variant of your own that stands for "directory":

```rust
let mut scanner = ScannerBuilder::<MyType>::new()
    .add_extractor(MyType::Folder, FolderExtractor::<MyType>::new(true, false)) // recursive, shared file opens
    .add_identifier(MyType::Png, PngIdentifier {})
    .add_analyzer(MyType::Png, 0, PngAnalyzer {})
    .build();

let mut root = FolderContent::<MyType>::with_content_type("C:/pictures", MyType::Folder);
let res = scanner.scan(&mut root, false);
```

Behaviour worth knowing:

- The `recursive` flag passed to `new` decides whether subdirectories are emitted at all. When `false`, only files directly inside the folder are scanned.
- Subdirectories are emitted as `FolderContent` carrying the **same** content type as their parent, so the same extractor picks them up again. Files become `FileContent` built with `with_size`, reusing the size from the directory entry's metadata. Each entry's path is filled with `ContentPath::set_from_os`, so non-UTF-8 filesystem names stay openable.
- Directory symlinks (and, on Windows, directory junctions / reparse points) are skipped, which keeps cyclic link structures from looping forever. Symlinks to regular files are followed and scanned like any other file; dangling links are skipped.
- An unreadable entry (permission error, failed `file_type`) is skipped; the rest of the directory is still enumerated.
- Subdirectory entries are marked `skip_from_filtering`, so an extension-based `Filter` narrows down the files without preventing the walk from descending.
- Recursion is still bounded by the scanner's `max_depth` (default `8`), which here translates into directory nesting levels.

### `Filter` / `FilterBuilder`

Filters decide whether a given `(ContentPath, size)` should be processed. They combine typed rules (extension / file-name allow- and deny-lists, powered by fast matchers) with arbitrary predicate callbacks, plus a default fallback:

```rust
use content_scan::*;

let filter = FilterBuilder::new()
    .include_extensions(Precedence::High,   &["rs", "toml"])
    .exclude_file_names(Precedence::Medium, &["Cargo.lock"])
    .exclude(Precedence::Low, |_path, size| size > 10 * 1024 * 1024) // skip huge files
    .deny_the_rest()   // or .allow_the_rest()
    .build();

let scanner = ScannerBuilder::<MyType>::new()
    .filter(filter)
    // ...
    .build();
```

Available builder methods:

| Method                                          | Effect                                    |
| ----------------------------------------------- | ----------------------------------------- |
| `include_extensions(prec, &["ext", …])`         | Allow if the file's extension matches (ASCII case-insensitive). |
| `exclude_extensions(prec, &["ext", …])`         | Deny if the file's extension matches (ASCII case-insensitive).  |
| `include_file_names(prec, &["name", …])`        | Allow if the basename matches (ASCII case-insensitive).         |
| `exclude_file_names(prec, &["name", …])`        | Deny if the basename matches (ASCII case-insensitive).          |
| `include(prec, fn(&ContentPath, u64) -> bool)`  | Allow if the callback returns `true`.     |
| `exclude(prec, fn(&ContentPath, u64) -> bool)`  | Deny if the callback returns `true`.      |
| `deny_the_rest()` / `allow_the_rest()`          | Set the default and finalize the builder. |

Rules are grouped by `Precedence` and evaluated from `Highest` to `Lowest`. Within the same precedence, rules keep the order they were added. The first matching rule wins. Extension and file-name rules are ASCII case-insensitive (`Photo.JPG` matches `jpg`).

Filters are also applied to entries produced by extractors, so filtering works transparently for embedded content.

### `Scanner` / `ScannerBuilder`

Assemble a scanner with `ScannerBuilder`:

```rust
let scanner = ScannerBuilder::<MyType>::new()
    .filter(filter)                                   // optional
    .max_depth(8)                                     // default: 8
    .add_identifier(MyType::Text, TextIdentifier {})
    .add_analyzer(MyType::Text, 0, MyAnalyzer {})
    .add_generic_analyzer(10, LoggingAnalyzer {})
    .add_extractor(MyType::Zip, ZipExtractor::default())
    .build();
```

Then scan:

```rust
let result: ScanResult = scanner.scan(&mut content, /* filter_root */ true);
```

`max_depth` limits how deep the scanner is allowed to recurse into extracted children (default `8`, minimum `1`). The root is depth `1`; a child of a depth-`N` object is depth `N + 1`. Extraction stops when that next child would exceed `max_depth`, so `max_depth(8)` visits at most eight objects on any path.

The second argument to `scan` decides whether the configured `Filter` is applied to the root object itself. Pass `true` for a normal file — the scan then returns an empty `ScanResult` if the filter rejects it. Pass `false` when the root is a container that the filter was never written to accept, such as a folder being walked with a filter that only allows `png` files. Extracted children are always filtered regardless of this flag (unless their `Entry` opts out via `skip_from_filtering`).

A scanner is reusable: `scan` clears its internal `Context` on entry, so one instance can process many inputs in sequence.

### `Context` / `ScanResult`

The `Context` passed to analyzers (from the [`varmap`](https://crates.io/crates/varmap) crate, re-exported by `content_scan`) exposes two `VarMap`s plus a way to queue extra extraction:

- `context.global()` — persists for the entire `scan()` call. Use it to accumulate results across all analyzed objects.
- `context.local()` — per-object scratch storage. The first call from an analyzer on a given object lazily grabs a `VarMap` from an internal pool, clears it, and attaches it to that object; subsequent calls (from other analyzers running on the same object) return the same map. It is kept alive after the scan and can be looked up on the corresponding `ScanContentHandle` via `ScanResult::local(handle)`.
- `context.request_extract(ty)` — queues an extra extraction pass: after this object's own extractors run, the scanner will run extractors registered for `ty` on the current content. See [Requesting extraction](#requesting-extraction). The request queue is cleared at the start of every object's scan, including nested children.

`context.objects_scanned()` returns how many objects have been visited so far.

After `scan()` returns, you can read results from the `ScanResult<T>`. In addition to the classic aggregate view, it also exposes the full **tree of scanned objects**:

```rust
let res = scanner.scan(&mut content, true);
let sum = res.global().get::<u32>(var!("sum")).unwrap_or(0);
println!("scanned {} objects, sum = {}", res.objects_scanned(), sum);
```

`var!("name")` is a compile-time typed key macro provided by `varmap`. Custom types can be stored in a `VarMap` by deriving `VarMapValue`, which is re-exported from `content_scan` as well:

```rust
#[derive(Debug, Copy, Clone, Eq, PartialEq, VarMapValue)]
pub struct Size { pub width: u32, pub height: u32 }

context.local().set(var!("size"), Size { width, height });
```

### Navigating the scan result tree

Every object visited by the scanner is recorded, along with its resolved content type, its path (interned from `ContentPath::as_printable_string()` into an internal arena) and its optional local `VarMap`. Objects are linked as a **parent / first-child / next-sibling** tree that mirrors the extraction hierarchy.

You navigate the tree with opaque `ScanContentHandle`s returned by `ScanResult<T>`:

```rust
pub struct ScanContentHandle { /* opaque */ }

impl<'a, T: ContentType> ScanResult<'a, T> {
    pub fn global(&self) -> &VarMap;
    pub fn objects_scanned(&self) -> u32;

    // Tree navigation
    pub fn root(&self) -> Option<ScanContentHandle>;
    pub fn parent(&self, handle: ScanContentHandle) -> Option<ScanContentHandle>;
    pub fn child(&self, handle: ScanContentHandle) -> Option<ScanContentHandle>;
    pub fn next_sibling(&self, handle: ScanContentHandle) -> Option<ScanContentHandle>;

    // Per-object data
    pub fn path(&self, handle: ScanContentHandle) -> Option<&str>;
    pub fn content_type(&self, handle: ScanContentHandle) -> Option<T>;
    pub fn local(&self, handle: ScanContentHandle) -> Option<&VarMap>;
}
```

`ScanResult::path` is the interned printable view of the object's `ContentPath` (`as_printable_string()` at scan time). For a live content object, use `content.path().as_printable_string()` to display it or `content.path().as_path()` to open it.

Typical walk:

```rust
fn dump<T: ContentType>(res: &ScanResult<T>, h: ScanContentHandle, depth: usize) {
    let pad = "  ".repeat(depth);
    let path = res.path(h).unwrap_or("?");
    let ty   = res.content_type(h);
    println!("{pad}- {path} ({ty:?})");

    // walk children left-to-right via first-child / next-sibling
    let mut c = res.child(h);
    while let Some(cur) = c {
        dump(res, cur, depth + 1);
        c = res.next_sibling(cur);
    }
}

if let Some(root) = res.root() {
    dump(&res, root, 0);
}
```

Because paths and local `VarMap`s live in pools owned by the scanner's `Context`, they are reused across successive `scan()` calls with zero re-allocation once steady-state is reached — the `ScanResult` simply borrows them for the lifetime of the current scan.

---

## Scanning pipeline

For every scanned object, the scanner performs the following steps (see [`content_scan/src/scanner.rs`](content_scan/src/scanner.rs)):

1. **Top-level filter check.** If a `Filter` is configured *and* `scan` was called with `filter_root = true`, the root content is tested first; if rejected, the scan returns immediately with an empty result.
2. **Type resolution.** If the content already reports a `content_type()`, it is used as-is. Otherwise the scanner tries, in order, using `ContentPath::as_bytes()` for the name-based steps:
   1. magic bytes (first 16 bytes; `build` panics if a registered magic is longer),
   2. exact file name,
   3. file extension,
   4. identifiers that returned `None` from `identify_method` (each `validate()` is tried in registration order).

   Each fast-matcher candidate is confirmed via the corresponding identifier's `validate()` method. Custom identifiers have no pre-filter; `validate()` is the identification.
3. **Type-specific analyzers** for the resolved type run in priority order.
4. **Generic analyzers** run for every object in priority order.
5. **Type-specific extractors** for the resolved type run in registration order (`create_session` → `advance`/`extract` loop; the session is dropped when the loop ends). Each `create_session` receives an `OwnedContentPtr` to the parent and an `ExtractionContext` covering the whole object (`offset = 0`, `length = Some(size)`, `params = None`). For each entry they emit, the scanner recurses (subject to `max_depth` and `Filter`). Entries marked `skip_from_filtering` bypass the `Filter` check.
6. **Extraction requests** queued by analyzers via `context.request_extract(ty)` then run, in emission order. For each request the extractors registered for `ty` run on the **same** parent, with that request's offset, length, and params. The parent does not need to have been identified as `ty`.

While this is happening, the scanner also **records the object** into `Context::objects` — interned from `ContentPath::as_printable_string()` into an internal arena, tagged with the resolved content type, and linked into its parent's child list. After `scan()` returns, that tree is exposed to the caller through [`ScanResult`](#navigating-the-scan-result-tree).

Only **analyzers** return `NextAction`. Extractor and session methods return `Option` (`create_session` / `advance` / `extract`); they cannot short-circuit the scan themselves.

- Analyzer `Skip` on an object stops remaining analyzers and extractors **on that object**. The scanner maps that Skip to `Continue` for the parent, so the session that produced the object keeps enumerating siblings.
- Analyzer `Exit` aborts the whole scan. The extraction session that produced the current object is dropped as the call stack unwinds; remaining extractors on ancestors do not run.

---

## Building & testing

Standard Cargo workflow from the workspace root:

```bash
# build everything
cargo build

# run the unit tests
cargo test

# run an example
cargo run --example vowals
cargo run --example sum
cargo run --example image_size -- path/to/image.jpg
```

The workspace pins `resolver = "2"` and applies a couple of shared Clippy overrides (`module_inception`, `new_without_default`) — see the root [`Cargo.toml`](Cargo.toml).

---

## License

Licensed under the [MIT License](LICENSE). © 2026 Gavrilut Dragos.
