# Basic concepts

A `content_scan` program is always the same shape. You define the **kinds of content** your tool understands, you hand the scanner a **byte source**, and you plug in three kinds of plugin: **identifiers**, **analyzers**, and **extractors**. The scanner calls those plugins in a fixed order, recurses into children, and accumulates everything it learned in a **context**.

This chapter names those pieces. The rest of the book is the details.

## The pieces

**[`ContentType`](content_type.md)** is a user-defined enum: `Text`, `Png`, `Zip`, `Folder`, whatever *this* scanner cares about. Every `Scanner` is parameterized by that enum. Two applications can share the crate and still have completely different type sets.

**[`Content`](content.md)** is one object the scanner can look at: a path, a size, and a way to read bytes at an offset. The crate ships `BufferContent`, `FileContent`, and `FolderContent`. You can implement the trait for anything else. Each content object also has a [`ContentPath`](content_path.md) — a real OS path, or a synthetic address such as a name inside an archive.

**[`Identifier`](identifier.md)** answers *what type is this?* There is **at most one identifier per `ContentType`**. Registering a second one for the same variant panics in `ScannerBuilder::build`.

**[`Analyzer`](analyzer.md)** inspects an object that has already been typed (or, for generic analyzers, any object) and records information. You may register **many analyzers**, including several for the same type. They run in priority order. Analyzers do not return “the scan result”; they write into the context (and they can emit findings). How that storage works is the subject of [Chapter 4](../chapter-4/context.md).

**[`Extractor`](extractor.md)** turns a container into child `Content` items. You may register **many extractors**, including several for the same type. They run in registration order. Each child is scanned recursively, up to `max_depth`.

Around those sit a **`Filter`** (optional include/exclude rules), the **`Scanner`** that owns the plugins, and the **`ScanResult`** you read when `scan()` returns.

## How they fit

The architecture page is the map: one object through the pipeline, many plugins of each kind, and where data lands.

- [Architecture](architecture.md) — pipeline schema, plugin cardinality, context and findings as a general notion.
- [Content](content.md) — the byte source, then `ContentType` and `ContentPath`.
- [Identifier](identifier.md) — magic, name, extension, and `validate`.
- [Analyzer](analyzer.md) — `NextAction`, priority, generic vs typed, [dependencies](dependencies.md).
- [Extractor](extractor.md) — sessions, `Entry`, nested containers.
    - [Extraction Context](extraction_context.md) — the window `create_session` receives.
    - [Requesting extraction](requesting_extraction.md) — analyzers queue extractors of another type.
    - [Extractions vs Context](extractions_vs_context.md) — small structs on the parent vs child `Content`.

[Chapter 5](../chapter-5/pipeline.md) walks the same pipeline with the exact control-flow rules (`Skip` / `Exit`, requested extraction, `filter_root`). This chapter stays at the model.
