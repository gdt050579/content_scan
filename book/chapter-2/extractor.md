# Extractor

An extractor **produces children**. Identifiers name the parent; analyzers inspect it; extractors open it and yield new [`Content`](content.md) objects, which the scanner then runs through the same pipeline (filter → identify → analyze → extract) until [`max_depth`](../chapter-3/recursion.md).

Extractors do not write the [`Context`](../chapter-4/context.md) and they do not return [`NextAction`](analyzer.md#nextaction). They yield `Option`. Steering the scan is still the analyzers’ job.

You may register **many** extractors, including several for the same type. There is no generic extractor: every one is keyed by a `ContentType`.

## Two types, not one

The design is a **shared extractor** plus a **per-parent session**.

```rust
pub trait ContentExtractor<T: ContentType> {
    fn create_session(
        &mut self,
        content: OwnedContentPtr<T>,
        extract_context: &ExtractionContext,
    ) -> Option<Box<dyn ExtractionSession<T>>>;
}

pub trait ExtractionSession<T: ContentType> {
    fn advance(&mut self) -> Option<&Entry>;
    fn extract(&mut self) -> Option<Box<dyn Content<T>>>;
}
```

One extractor instance lives on the `Scanner` and is reused for every object of that type — including nested containers. `create_session` can be called again while a previous session is still live (a ZIP inside a folder, a ZIP inside a ZIP). **Configuration** stays on the extractor (`recursive`, exclusive file opens, …). **Cursors, open archives, and the current `Entry`** stay on the session.

Put a `pos` or a `ZipArchive` on the extractor itself and a nested extraction will overwrite the outer one. That is the bug this split exists to prevent.

`create_session` returns `None` to skip this extractor; the scanner moves on to the next one registered for the same type. Implement `Drop` on the session if you need to close files or delete temp buffers when enumeration ends.

## When an extractor runs

An extractor registered for type `T` runs in two situations, **after** that object’s analyzers have finished (and only if they returned `Continue`, not `Skip` / `Exit`):

1. **The parent was identified as `T`.** `ExtractionContext` covers the whole object: `offset = 0`, `length = Some(content.size())`, `params = None`.
2. **An analyzer requested `T`** with `context.request_extract(T)`. The parent does **not** need to have been identified as `T`. The context then carries that request’s offset, length, and params. See [Requesting extraction](requesting_extraction.md).

Order: extractors for the object’s **own** type first (registration order), then requested types in **emit** order. Several extractors for the same type all run, in the order they were added — there is no priority byte.

```rust
.add_extractor(MyTypes::Zip, ZipExtractor::new())
.add_extractor(MyTypes::Zip, ZipExtraStreamExtractor {})
.add_extractor(MyTypes::Folder, FolderExtractor::<MyTypes>::new(true, false))
.add_extractor(MyTypes::Text, NumericExtractor)
```

## The session loop

The scanner drives each session as:

```text
     create_session(parent, ExtractionContext)
            │
            │ None → skip this extractor
            ▼
     ┌─────────────────────────────────┐
     │ advance() → Option<&Entry>      │
     │                                 │
     │  None  → drop session (done)    │
     │  Some  → filter on path / size  │
     │          (unless skip_from_     │
     │           filtering)            │
     │          reject → on_filtered   │
     └────────────┬────────────────────┘
                  │ keep → on_extraction
                  ▼
     extract() → Option<Box<dyn Content>>
            │
            │ None → skip this entry, advance again
            ▼
     inner_scan(child)   depth+1, up to max_depth
            │
            │ child's Skip  → continue advance
            │ child's Exit  → drop session, abort scan
            ▼
         advance() again
```

`advance` is cheap: path, size, and a filter flag. The scanner can reject an entry **before** you decompress or allocate the child. `extract` is the expensive step: build a `BufferContent`, `FileContent`, or your own `Content`.

The window the session looks at is an [`ExtractionContext`](extraction_context.md) — whole object, or a slice an analyzer [requested](requesting_extraction.md). When a value should stay on the parent instead of becoming a child, use the scan [`Context`](extractions_vs_context.md), not an extractor.

A child’s `Skip` does not end the session. A child’s `Exit` drops it as the scan unwinds. An [observer](../chapter-3/observer.md) sees a filter reject as `on_filtered` and a kept entry as `on_extraction`, just before `extract()`.

## `Entry`

```rust
pub struct Entry {
    pub path: ContentPath,
    pub size: u64,
    pub skip_from_filtering: bool,
}
```

Keep **one** `Entry` on the session and overwrite `entry.path` in place (`set_from_str` for synthetic names, `set_from_os` for real filesystem paths) so a hot loop does not allocate a new path per child. See [ContentPath](content_path.md).

`skip_from_filtering` exempts the entry from the active [`Filter`](../chapter-3/filter.md). Subdirectories from `FolderExtractor` set it so an extension-only filter (`*.png`) still lets the walk descend into folders that have no `.png` in their name.

## `OwnedContentPtr` and `ContentReader`

`create_session` receives an `OwnedContentPtr<T>`, not `&mut dyn Content<T>`. It does **not** own the parent: it is a handle that derefs to `Content` so the session can `read` / `size` / `path` across `advance` and `extract` without fighting the borrow checker. The scanner guarantees the parent outlives the session. Do not store the pointer after the session is dropped.

[`Content::read`](content.md) is random-access and returns a borrowed slice. Libraries that want a `std::io::Read` + `Seek` stream (the `zip` crate, parsers, decompressors) wrap the handle:

```rust
fn create_session(
    &mut self,
    content: OwnedContentPtr<MyTypes>,
    _: &ExtractionContext,
) -> Option<Box<dyn ExtractionSession<MyTypes>>> {
    let archive = zip::ZipArchive::new(ContentReader::new(content)).ok()?;
    Some(Box::new(MyZipSession { archive, entry: Entry::default(), /* ... */ }))
}
```

A short `Content::read` is not EOF for `ContentReader`: it copies what it got and continues. `None` before `size()` becomes `UnexpectedEof`.

## Example: numbers out of text

The `sum` example is the smallest complete extractor. A `Text` object is identified by magic `TXT`. The extractor walks the parent, yields each digit run as a child `BufferContent` pinned as `Number`, and a typed analyzer sums them.

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
        // scan forward for the next digit run, update pos / start / len ...
        self.entry.path.set_from_str("number");
        self.entry.size = self.len;
        self.entry.skip_from_filtering = false;
        Some(&self.entry)
    }

    fn extract(&mut self) -> Option<Box<dyn Content<MyTypes>>> {
        let buf = self.content.read(self.start, self.len as u32)?;
        Some(Box::new(BufferContent::<MyTypes>::with_content_type(
            buf,
            "number",
            MyTypes::Number,
        )))
    }
}
```

The children are **pinned** as `Number`, so identifiers do not run on them. Pin when you already know the type; leave it unset when the child’s magic or name should decide (ZIP members, directory files).

`cursor` and `entry` live on `NumericSession`. `NumericExtractor` is a unit struct: nested text containers would each get their own session.

## Built-ins

The crate ships two extractors. Their full behaviour is [Chapter 5](../chapter-5/builtins.md).

**`FolderExtractor`** — register it for your `Folder` variant, pair it with `FolderContent`. Files become `FileContent`; subfolders become `FolderContent` of the same type (that is what makes the walk recurse). Directory symlinks are skipped so cycles cannot loop. Subfolder entries set `skip_from_filtering`.

**`ZipExtractor`** — pair with `ZipIdentifier`. Members become `BufferContent` if they are under 1 MiB, otherwise a temp `FileContent`. The session reads the parent through `ContentReader`, so the archive can be a file, a buffer, or any other `Content`. Directory entries inside the ZIP are skipped.

## Recursion and filters

Each extracted child is a new object at `depth + 1`. Extraction stops when that next child would exceed `max_depth` (default 8; the root is depth 1). The filter, if any, is applied to each `Entry` unless `skip_from_filtering` is set. The root is filtered only when `scan(..., filter_root)` is `true` — [Recursion and filter_root](../chapter-3/recursion.md).

That is the last of the three plugin kinds. [Extraction Context](extraction_context.md), [Requesting extraction](requesting_extraction.md), and [Extractions vs Context](extractions_vs_context.md) complete the picture. [Architecture](architecture.md) is the map; [How one scan runs](../chapter-3/how_one_scan_runs.md) is the same loop with every `Skip` / `Exit` / requested-extraction edge.
