# Filter

A `Filter` decides whether a `(ContentPath, size)` pair should be scanned at all. It runs **before** identifiers, analyzers, and extractors. A reject means the object is not recorded and no plugins run on it. If an [observer](observer.md) is attached, that reject is `on_filtered` — the object never appears as `on_scan_object`.

Build one with `FilterBuilder`, then hand it to the scanner:

```rust
let filter = FilterBuilder::new()
    .include_extensions(Precedence::Medium, &["rs", "toml"])
    .exclude_file_names(Precedence::Medium, &["Cargo.lock"])
    .exclude(Precedence::Low, |_path, size| size > 10 * 1024 * 1024)
    .deny_the_rest()
    .build();

let scanner = ScannerBuilder::<MyTypes>::new()
    .filter(filter)
    // ...
    .build();
```

You must end with **`deny_the_rest()`** or **`allow_the_rest()`** before `build()`. That choice is the default when no rule matches, and the type system makes you pick it at the call site.

## Where it applies

- **Root** — only if `scan(..., filter_root)` is `true`. See [Recursion and filter_root](recursion.md).
- **Extracted children** — always, unless the session set `Entry::skip_from_filtering` (folders in a directory walk).

There is no per-type filter. Path and size are all it sees. Identification has not run yet.

## Rules

| Method                                         | When it fires                                      | Outcome if the callback/matcher hits |
| ---------------------------------------------- | -------------------------------------------------- | ------------------------------------ |
| `include_extensions(prec, &["ext", …])`        | Basename extension, no dot, ASCII case-insensitive | Allow                                |
| `exclude_extensions(prec, &["ext", …])`        | Same                                               | Deny                                 |
| `include_file_names(prec, &["name", …])`       | Basename, ASCII case-insensitive                   | Allow                                |
| `exclude_file_names(prec, &["name", …])`       | Same                                               | Deny                                 |
| `include(prec, fn(&ContentPath, u64) -> bool)` | Callback returns `true`                            | Allow (`false` = try the next rule)  |
| `exclude(prec, fn(&ContentPath, u64) -> bool)` | Callback returns `true`                            | Deny (`false` = try the next rule)   |

`Photo.JPG` matches `jpg`. Paths are inspected via [`ContentPath::as_bytes`](../chapter-2/content_path.md) (faithful on Unix; printable bytes on Windows).

## Precedence

Rules are grouped by `Precedence` and evaluated from **`Highest` to `Lowest`**. Within the same tier, order is the order they were added. **The first matching rule wins.**

```rust
Precedence::Highest
Precedence::High
Precedence::Medium   // typical for extension lists
Precedence::Low
Precedence::Lowest
```

A high-precedence exclude of `Cargo.lock` beats a medium include of `*.toml` if both could apply. Put exceptions in a higher tier than the broad allow/deny.

Callbacks that return `false` are not a decision; later rules still run. Only `true` (include or exclude) stops the chain. If nothing matches, `deny_the_rest` / `allow_the_rest` is the answer.

## Compiled matchers

Extension and file-name lists become the same kind of matcher identifiers use (one pattern, packed table, or trie). Evaluating a filter on every ZIP member is meant to be cheap. Custom `include` / `exclude` closures are `fn` pointers, not capturing closures — they cannot borrow local state.

A scanner without `.filter(...)` accepts everything. Adding a filter and then passing `filter_root = false` on a folder still filters the files inside. Rejected children are visible to an observer as `on_filtered`; they are not nodes in `ScanResult`.
