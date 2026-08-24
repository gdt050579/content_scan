# ContentPath

Every content object has a name: `Content::path()` returns a `ContentPath`. Filters, name/extension identification, logging, and the scan-result tree all go through it.

A `String` or `PathBuf` is not enough. Some objects are not files (`archive.zip://inner.txt`, `"number"`). Some files have names that are **not valid UTF-8**, and you still need to open them. `ContentPath` always has a printable UTF-8 view, and it keeps the original OS path when that view would otherwise be lossy.

## Two ways to construct it

| Constructor                  | Use for                                                                       | UTF-8                                                                                 |
| ---------------------------- | ----------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| `ContentPath::from_str(s)`   | Synthetic addresses, labels you already know are UTF-8                        | Always lossless                                                                       |
| `ContentPath::from_os(path)` | Real filesystem paths (`DirEntry`, `PathBuf`, `FileContent`, `FolderContent`) | Lossless if the OS name is UTF-8; otherwise stores the `OsString` plus a lossy string |

Do **not** stringify a real OS path and pass it to `from_str`. On a non-UTF-8 name you would throw away the bytes needed to reopen the file. `FileContent` and `FolderContent` take `impl AsRef<Path>` and call `from_os` for you. `BufferContent` takes a `&str` and calls `from_str`, which is correct for a synthetic label.

`From` is implemented for `&str`, `String`, `&Path`, `PathBuf`, and their references. Strings go through `from_str` (or `with_string` when already owned); OS paths go through `from_os`.

## Three views

```rust
impl ContentPath {
    pub fn as_printable_string(&self) -> &str; // always valid UTF-8
    pub fn as_path(&self) -> &Path;            // openable OS path
    pub fn as_bytes(&self) -> &[u8];           // filtering / identification
    pub fn is_lossless(&self) -> bool;
}
```

**`as_printable_string`** — safe to print, log, or intern. Never fails. For a non-UTF-8 OS path this is the lossy rendering (`U+FFFD` for invalid sequences). The scan result tree stores this view: `ScanResult::path` is UTF-8 text, not a `Path`.

**`as_path`** — what you pass to `File::open`, `fs::read_dir`, and `FileContent`’s opener. For a lossless path this is the string reinterpreted as a `Path`. For a non-UTF-8 path it is the preserved `OsString`, so the original file is still named. Synthetic addresses (`zip://…`) also return a `&Path`; opening them as files simply fails at the OS.

**`as_bytes`** — what the **filter** and **identifiers** inspect (basename, extension). On Unix these are the faithful path bytes (including a non-UTF-8 `OsString`). On Windows the OS encoding is not exposable as bytes, so this is the printable string’s bytes.

**`is_lossless`** — `true` when the printable string is exact. `false` means “do not use the printable form as an identity or a key; open via `as_path()`.”

```text
            ┌────────────────────────────────────┐
            │          ContentPath               │
            │                                    │
            │  printable String  (always UTF-8)  │──── as_printable_string()
            │  optional OsString (if lossy)      │──── as_path()
            └────────────────────────────────────┘
                               │
                               └── as_bytes()   filter + IdentifyMethod
```

## Reusing a path in a session

Extractors enumerate many children. Allocating a new `ContentPath` per `Entry` is wasteful. Keep one `Entry` on the session and overwrite the path in place:

```rust
impl ExtractionSession<MyTypes> for MySession {
    fn advance(&mut self) -> Option<&Entry> {
        self.entry.path.clear();
        self.entry.path.set_from_str("number");      // synthetic
        // self.entry.path.set_from_os(dirent.path()); // real OS path
        self.entry.size = self.len;
        Some(&self.entry)
    }
    // ...
}
```

`empty()` / `clear()` keep the string allocation. `set_from_str` and `set_from_os` are the in-place counterparts of `from_str` and `from_os`. `FolderExtractor` fills each entry with `set_from_os`, so a directory walk does not drop non-UTF-8 names.

## Who looks at which view

| Consumer                                          | View                                                             |
| ------------------------------------------------- | ---------------------------------------------------------------- |
| `Filter` (extensions, file names, callbacks)      | `as_bytes()` for matching; callbacks also receive `&ContentPath` |
| Identifiers (`Name`, `Extension`)                 | `as_bytes()` of the path (basename / extension)                  |
| Analyzers / logging                               | `as_printable_string()`                                          |
| Opening a file (`FileContent`, `FolderExtractor`) | `as_path()`                                                      |
| Result tree (`ScanResult::path`)                  | interned `as_printable_string()`                                 |

A ZIP member named `photos/cat.png` is a synthetic path (`from_str` / `set_from_str`). A file walked from disk is `from_os`. Both can still match `Extension("png")` if the basename bytes look like a `.png` name.

## `Debug`

`Debug` prints the printable string and, when the path is lossy, a `lossy` flag, so a non-UTF-8 path is not silently pretty-printed as if it were exact.

That is the whole name of a content object. Combined with [`ContentType`](content_type.md) and `read` / `size`, it is everything the scanner needs before plugins run.
