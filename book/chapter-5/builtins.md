# Built-in plugins

The crate is a framework, not a format catalog. You still define a [`ContentType`](../chapter-2/content_type.md) enum and the analyzers that know your domain. What ships ready-made is the **transport** around that: byte sources, a directory walk, and ZIP identify/extract.

Nothing here is special-cased in the scanner. `FolderExtractor` and `ZipExtractor` are ordinary [`ContentExtractor`](../chapter-2/extractor.md)s. `ZipIdentifier` is an ordinary [`ContentIdentifier`](../chapter-2/identifier.md). You register them on [`ScannerBuilder`](../chapter-3/builder.md) against **your** variants (`MyTypes::Folder`, `MyTypes::Zip`).

## Ready-made `Content`

These are the roots you pass to `scan()`, and often the children extractors emit. Full constructors live in [Content](../chapter-2/content.md).

| Type | What it is |
| --- | --- |
| `BufferContent<T>` | Owned `Vec<u8>` plus a synthetic UTF-8 path. First-scan buffers, small ZIP members, decoded payloads. |
| `FileContent<T>` | A file on disk, opened lazily on the first `read()`. `exclusive` chooses mmap+lock vs shared LRU. |
| `FolderContent<T>` | A directory **marker**: `size()` is `0`, `read()` is `None`, type is pinned. Not a byte stream. |

Pick the root from what you have: bytes → buffer, path to a file → `FileContent`, path to a directory → `FolderContent` plus the [folder extractor](folder.md).

## Plugins in this chapter

| Piece | Trait | Job |
| --- | --- | --- |
| [`FolderContent` / `FolderExtractor`](folder.md) | `Content` + `ContentExtractor` | Walk a directory tree as nested scan objects. |
| [`ZipIdentifier` / `ZipExtractor`](zip.md) | `ContentIdentifier` + `ContentExtractor` | Recognize ZIP by content and unpack members. |

There is no built-in PNG/PE/PDF analyzer. [Examples](../chapter-6/examples.md) show those as application plugins on top of the folder and ZIP built-ins.

## `ContentReader`

[`Content::read`](../chapter-2/content.md) is random-access and returns a borrowed slice. Libraries that want `std::io::Read` + `Seek` (the `zip` crate, decompressors, streaming parsers) need a cursor instead.

`ContentReader<T>` wraps an [`OwnedContentPtr`](../chapter-2/extractor.md), starts at offset `0`, and copies each `Content::read` into the caller’s buffer. A short slice is not EOF — it copies what it got and continues. `None` before `size()` becomes `UnexpectedEof`. Seeking past the end is allowed (same idea as `std::io::Cursor`).

The [ZIP extractor](zip.md) is built on this. Your own extractor can do the same:

```rust
let archive = zip::ZipArchive::new(ContentReader::new(parent)).ok()?;
```

## Chapter map

- [Folder](folder.md) — `FolderContent`, `FolderExtractor`, `filter_root`, symlinks, `skip_from_filtering`.
- [ZIP](zip.md) — `ZipIdentifier` (magic + EOCD), `ZipExtractor` (members, temp files), combining with a folder walk.
