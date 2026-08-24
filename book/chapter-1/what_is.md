# What is Content Scanner

`content_scan` is a small, extensible **content scanning framework** for Rust. You describe the kinds of content your tool understands, plug in identifiers, analyzers, and extractors, and the [`Scanner`](../chapter-3/scanner.md) takes care of dispatch, recursion, filtering, and result aggregation.

The crate is transport-agnostic. Anything that implements [`Content`](../chapter-2/content.md) — an in-memory buffer, a file, a directory, an archive member, or your own source — can be the root of a scan.

The current release line is early and experimental (`0.1.x`). APIs can still change.

## The problem it solves

A lot of tools need the same skeleton:

1. Look at a blob of bytes (or a folder of them) and decide **what** it is.
2. Run one or more **analyses** on that object (metrics, hashes, findings, …).
3. If the object is a **container**, pull children out of it and repeat, with a depth limit and an optional filter.

Writing that loop once per project — type dispatch, nested archives, directory walks, “skip this file” rules, “abort the whole scan” — is tedious and easy to get wrong. `content_scan` is that loop, as a library. The domain knowledge stays in *your* plugins.

Typical uses:

- File-type detection from magic bytes, extensions, or file names.
- Static analysis pipelines (metrics, heuristics, feature extraction).
- Recursive scanning of containers (archives, bundles, embedded blobs).
- Walking a directory tree (`FolderContent` + `FolderExtractor`).
- Building custom scanners (indexers, linters, forensics tools, antivirus-like pipelines) on a common core.

It is **not** a ready-made virus scanner, MIME database, or file-format catalog. Those would be plugins you register. The crate ships a few built-ins (folder walk, ZIP identify/extract) so real scans can start without writing every adapter yourself.

## Identify, analyze, extract

Three plugin kinds do the work. The scanner decides *when* they run.

| Plugin     | Trait               | Job                                                                                                        |
| ---------- | ------------------- | ---------------------------------------------------------------------------------------------------------- |
| Identifier | `ContentIdentifier` | Classify an object into one of your `ContentType` variants (magic, name, extension, or custom `validate`). |
| Analyzer   | `ContentAnalyzer`   | Inspect the object and record information into a shared [`Context`](../chapter-4/context.md).              |
| Extractor  | `ContentExtractor`  | Open a session on a parent and yield child `Content` items, which the scanner then scans recursively.      |

You also define a [`ContentType`](../chapter-2/content_type.md) enum: the closed set of kinds *this* scanner knows about. Every `Scanner` is parameterized by that enum. Two applications can use the crate with completely different type sets.

Around those plugins sit:

- **`Content` / `ContentPath`** — the byte source and its name (a real OS path or a synthetic address such as `archive.zip://inner.txt`).
- **`Filter`** — optional include/exclude rules applied before plugins run.
- **`Scanner` / `ScannerBuilder`** — registration, `max_depth`, and `scan()`.
- **`ScanResult`** — after the scan, a tree of visited objects, per-object and scan-wide maps, and a flat list of findings.

Chapter 2 defines these pieces. [How one scan runs](../chapter-3/how_one_scan_runs.md) walks one object through the pipeline as the scanner implements it. For now, the important picture is: **you write plugins; the scanner calls them in a fixed order and builds a result tree.**

## What you write versus what the scanner does

You write:

- The `ContentType` enum.
- Identifiers, analyzers, and extractors (or you reuse the built-in folder and ZIP plugins).
- The `Content` you hand to `scan()` (`BufferContent`, `FileContent`, `FolderContent`, or your own).

The scanner:

- Applies the filter (when configured).
- Resolves the type, unless the content already reports one.
- Runs type-specific analyzers, then generic analyzers.
- Runs extractors for the resolved type, then any extra extractions analyzers requested.
- Recurses into children up to `max_depth`.
- Records each object in a parent/child/sibling tree you can walk afterwards.

Analyzers steer that process with `NextAction`: continue this object, skip the rest of it, or abort the entire scan.

## Who this book is for

This book is for people building a tool *on top of* the framework — or reading the crate to understand why a scan behaved a certain way. It is not a replacement for rustdoc. When a later chapter quotes an API, treat the source and docs as the contract; the book is the narrative of how those APIs fit together.
