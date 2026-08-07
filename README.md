# content_scan

A small, extensible **content scanning framework** for Rust.

`content_scan` gives you a plug-in style pipeline for **identifying**, **analyzing** and **extracting** structured content out of arbitrary byte streams (files, buffers, embedded archives, etc.). You define your own content types and plug in identifiers, analyzers and extractors — the scanner takes care of dispatching, recursion and filtering.

Typical use cases:

- File-type / MIME-like detection based on magic bytes, extensions or file names.
- Static analysis pipelines (metrics, heuristics, feature extraction).
- Recursive scanning of container formats (archives, bundles, embedded blobs) with a configurable depth limit.
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
  - [API overview](#api-overview)
    - [`ContentType`](#contenttype)
    - [`Content`](#content)
    - [`ContentIdentifier`](#contentidentifier)
    - [`ContentAnalyzer`](#contentanalyzer)
    - [`ContentExtractor`](#contentextractor)
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
| `examples`                | [`examples/`](examples)                               | Runnable examples (`sum`, `vowals`).                                                           |

You normally only depend on `content_scan` — the proc-macro is re-exported for you.

---

## Core concepts

The framework is built around a few small traits:

- **`ContentType`** — a `#[repr(u16)]` enum describing the kinds of content your scanner knows about. Derived with `#[derive(ContentType)]`.
- **`Content<T>`** — an abstract, seekable, read-only byte source with a path and a size. A ready-made `BufferContent<T>` is provided for in-memory buffers.
- **`ContentIdentifier<T>`** — decides *what* a piece of content is (by magic bytes, extension, or file name) and validates the guess.
- **`ContentAnalyzer<T>`** — reads content and produces information (stored in a shared `Context`).
- **`ContentExtractor<T>`** — pulls sub-contents out of a container and hands them back to the scanner, which recurses into them.
- **`Filter`** — decides which paths / sizes should be processed at all.
- **`Scanner<T>`** — the orchestrator; built via `ScannerBuilder<T>`.
- **`ScanResult<T>` / `ScanContentHandle`** — after a scan, the framework exposes the full **tree** of visited objects (parent / child / sibling links), each with its interned path, resolved content type and its own local `VarMap`.

Analyzers and extractors are either **specific** to a `ContentType` or **generic** (run on every scanned object). Each is registered with a `priority` byte to control execution order.

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

#[derive(Debug, Copy, Clone, Eq, PartialEq, ContentType)]
#[repr(u16)]
enum MyType {
    TextBuffer,
}

// ... implement ContentIdentifier / ContentAnalyzer / ContentExtractor ...

fn main() {
    let mut scanner = ScannerBuilder::<MyType>::new()
        // .add_identifier(MyType::TextBuffer, MyIdentifier {})
        // .add_analyzer(MyType::TextBuffer, 0, MyAnalyzer {})
        // .add_extractor(MyType::TextBuffer, 0, MyExtractor::default())
        .build();

    let mut content = BufferContent::<MyType>::new(b"...", "input.bin");
    let result = scanner.scan(&mut content);

    println!("scanned {} objects", result.objects_scanned());
}
```

---

## Examples

The [`examples/`](examples) directory contains runnable programs. From the workspace root:

```bash
cargo run --example vowals
cargo run --example sum
```

### Counting vowels

A single analyzer that counts vowels in a magic-tagged buffer. See [`examples/vowals/main.rs`](examples/vowals/main.rs).

```rust
use content_scan::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq, ContentType)]
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
    fn validate(&self, _: &dyn Content<MyType>) -> bool { true }
}

fn main() {
    let mut scanner = ScannerBuilder::new()
        .add_analyzer(MyType::TextBuffer, 0, VowelAnalyzer {})
        .add_identifier(MyType::TextBuffer, TextBufferIdentifier {})
        .build();

    let mut b = BufferContent::<MyType>::with_content_type(
        b"TXBF   Hellow World !", "test.txt", MyType::TextBuffer,
    );
    let res = scanner.scan(&mut b);
    println!("count_vowels: {}", res.global().get::<u32>(var!("count_vowels")).unwrap());
}
```

### Summing numbers extracted from text

This example shows the full pipeline — a text container is *identified* by magic bytes, an *extractor* pulls numeric substrings out of it as independent `Number` contents, and a numeric *analyzer* both sums them into a shared **global** variable and stashes each value into a per-object **local** `VarMap`. After the scan, the example walks the resulting tree via `ScanResult`. See [`examples/sum/main.rs`](examples/sum/main.rs).

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
        .add_extractor(MyTypes::Text, 0, NumericExtractor::default())
        .add_identifier(MyTypes::Text, TextIdentifier {})
        .build();

    let mut b = BufferContent::<MyTypes>::new(b"TXT   1+2+3=", "test.txt");
    let res = scanner.scan(&mut b);
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

---

## API overview

### `ContentType`

Every scanner is parameterized by a user-defined enum implementing `ContentType`. The `#[derive(ContentType)]` macro implements the trait automatically. Requirements:

- The enum must be annotated with `#[repr(u16)]`.
- Variants must be **unit variants** with **no explicit discriminants**.
- Up to `65536` variants are supported.

```rust
#[derive(Debug, Copy, Clone, Eq, PartialEq, ContentType)]
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
    fn path(&self) -> &str;
    fn size(&self) -> u64;
    fn read(&mut self, offset: u64, count: u32) -> Option<&[u8]>;
}
```

A `Content` is any addressable byte source. It exposes a logical `path` (used for identification and filtering), a `size`, and a `read(offset, count)` method that returns a borrowed slice.

An in-memory implementation is provided:

```rust
BufferContent::<MyType>::new(buffer, "path.ext");
BufferContent::<MyType>::with_content_type(buffer, "path.ext", MyType::Text);
BufferContent::<MyType>::from_parts(vec, "path".into(), Some(MyType::Text));
```

You can implement `Content<T>` for files, memory-mapped regions, network streams, archive entries, etc.

### `ContentIdentifier`

Identifiers tell the scanner *what type* a piece of content is when the content itself doesn't already report one.

```rust
pub trait ContentIdentifier<T: ContentType> {
    fn identify_method(&self) -> Option<IdentifyMethod>;
    fn validate(&self, content: &dyn Content<T>) -> bool;
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

Fast identification is performed with an internal matcher (single-pattern, packed magic table, or trie depending on the number and shape of patterns). After a fast match, `validate()` is called to confirm the guess.

At most **one identifier per `ContentType`** may be registered; the builder will panic otherwise.

### `ContentAnalyzer`

```rust
pub trait ContentAnalyzer<T: ContentType> {
    fn analyze(&mut self, content: &mut dyn Content<T>, context: &mut Context) -> NextAction;
}
```

Analyzers inspect content and write results into the shared `Context`. The returned `NextAction` controls the pipeline:

- `NextAction::Continue` — run the next analyzer / extractor.
- `NextAction::Skip` — stop processing the current object (do not run further analyzers/extractors on it), but keep scanning siblings.
- `NextAction::Exit` — stop the whole scan.

Register analyzers with:

- `add_analyzer(content_type, priority, analyzer)` — runs only when the content matches `content_type`.
- `add_generic_analyzer(priority, analyzer)` — runs for every scanned object, regardless of type.

Within a bucket, analyzers execute in ascending `priority` order.

### `ContentExtractor`

```rust
pub trait ContentExtractor<T: ContentType> {
    fn init(&mut self, content: &mut dyn Content<T>, extract_context: &mut VarMap) -> bool;
    fn advance(&mut self, content: &mut dyn Content<T>) -> Option<&Entry>;
    fn extract(&mut self, content: &mut dyn Content<T>) -> Option<Box<dyn Content<T>>>;
}

pub struct Entry {
    pub path: String,
    pub size: u64,
}
```

Extractors turn a container into a stream of children:

1. `init` — called once, receives a scratch `VarMap` (`context.extract()`) valid until the next object is scanned. Return `false` to skip this extractor.
2. `advance` — advances to the next child and returns a lightweight `Entry` describing its path/size. Returning `None` ends the stream.
3. `extract` — materializes the current child as a boxed `Content<T>`. The scanner then recursively scans it (subject to `max_depth`).

Extractors are registered with `add_extractor` / `add_generic_extractor`, mirroring analyzers.

### `Filter` / `FilterBuilder`

Filters decide whether a given `(path, size)` should be processed. They combine typed rules (extension / file-name allow- and deny-lists, powered by fast matchers) with arbitrary predicate callbacks, plus a default fallback:

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

| Method                                   | Effect                                    |
| ---------------------------------------- | ----------------------------------------- |
| `include_extensions(prec, &["ext", …])`  | Allow if the file's extension matches.    |
| `exclude_extensions(prec, &["ext", …])`  | Deny if the file's extension matches.     |
| `include_file_names(prec, &["name", …])` | Allow if the file name matches exactly.   |
| `exclude_file_names(prec, &["name", …])` | Deny if the file name matches exactly.    |
| `include(prec, fn(&str, u64) -> bool)`   | Allow if the callback returns `true`.     |
| `exclude(prec, fn(&str, u64) -> bool)`   | Deny if the callback returns `true`.      |
| `deny_the_rest()` / `allow_the_rest()`   | Set the default and finalize the builder. |

Rules are evaluated in the order they were added; the first matching rule wins. The `Precedence` enum is provided to make the intent explicit.

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
    .add_extractor(MyType::Zip, 0, ZipExtractor::default())
    .add_generic_extractor(100, FallbackExtractor::default())
    .build();
```

Then scan:

```rust
let result: ScanResult = scanner.scan(&mut content);
```

`max_depth` limits how deep the scanner is allowed to recurse into extracted children (default `8`, minimum `1`).

### `Context` / `ScanResult`

The `Context` passed to analyzers exposes three `VarMap`s (from the [`varmap`](https://crates.io/crates/varmap) crate, re-exported by `content_scan`):

- `context.global()` — persists for the entire `scan()` call. Use it to accumulate results across all analyzed objects.
- `context.local()` — per-object scratch storage. The first call from an analyzer on a given object lazily grabs a `VarMap` from an internal pool, clears it, and attaches it to that object; subsequent calls (from other analyzers running on the same object) return the same map. It is kept alive after the scan and can be looked up on the corresponding `ScanContentHandle` via `ScanResult::local(handle)`.
- `context.extract()` — a scratch map handed to a single extractor's `init` call.

`context.objects_scanned()` returns how many objects have been visited so far.

After `scan()` returns, you can read results from the `ScanResult<T>`. In addition to the classic aggregate view, it also exposes the full **tree of scanned objects**:

```rust
let res = scanner.scan(&mut content);
let sum = res.global().get::<u32>(var!("sum")).unwrap_or(0);
println!("scanned {} objects, sum = {}", res.objects_scanned(), sum);
```

`var!("name")` is a compile-time typed key macro provided by `varmap`.

### Navigating the scan result tree

Every object visited by the scanner is recorded, along with its resolved content type, its path (interned in an internal arena) and its optional local `VarMap`. Objects are linked as a **parent / first-child / next-sibling** tree that mirrors the extraction hierarchy.

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

1. **Top-level filter check.** If a `Filter` is configured, the root content is tested first; if rejected, the scan returns immediately.
2. **Type resolution.** If the content already reports a `content_type()`, it is used as-is. Otherwise the scanner tries, in order:
   1. magic bytes (first 16 bytes),
   2. exact file name,
   3. file extension.

   Each candidate is confirmed via the corresponding identifier's `validate()` method.
3. **Type-specific analyzers** for the resolved type run in priority order.
4. **Generic analyzers** run for every object in priority order.
5. **Type-specific extractors** run and, for each entry they emit, the scanner recurses (subject to `max_depth` and `Filter`).
6. **Generic extractors** run and recurse in the same way.

While this is happening, the scanner also **records the object** into `Context::objects` — allocating its path in an internal arena, tagging it with the resolved content type, and linking it into its parent's child list. After `scan()` returns, that tree is exposed to the caller through [`ScanResult`](#navigating-the-scan-result-tree).

Any analyzer or extractor may short-circuit the current object with `NextAction::Skip` or abort the entire scan with `NextAction::Exit`.

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
```

The workspace pins `resolver = "2"` and applies a couple of shared Clippy overrides (`module_inception`, `new_without_default`) — see the root [`Cargo.toml`](Cargo.toml).

---

## License

Licensed under the [MIT License](LICENSE). © 2026 Gavrilut Dragos.
