# Scanner

The `Scanner` is the pipeline engine. It owns the plugins, the optional filter, `max_depth`, and the [`Context`](../chapter-4/context.md) that a scan fills. You do not construct one by hand: [`ScannerBuilder`](builder.md) does that. You drive it with `scan()`.

```rust
let result = scanner.scan(&mut content, /* filter_root */ true);
```

What happens inside one call is [How one scan runs](how_one_scan_runs.md). This chapter is how you assemble the engine, how you reuse it, and the two knobs that most people get wrong: the [filter](filter.md) and [`filter_root` / `max_depth`](recursion.md).

## Create once, scan many times

A scanner is **built once** and **used for many inputs**. That is the intended lifecycle.

`build()` compiles identifier matchers, sorts analyzers by priority, and takes ownership of every plugin instance — including analyzers that loaded signatures or rule files in their constructor. Doing that per file would reload tables and rebuild tries on every path.

`scan()` is the cheap call. At the start of each one the scanner **clears its internal `Context`** (object tree, maps, findings, extraction-request queue) so nothing leaks from the previous input. Plugin instances are **not** reconstructed. A `SignatureAnalyzer` that called `from_file` at builder time still holds those rules on the hundredth `scan()`.

```rust
let mut scanner = ScannerBuilder::<MyTypes>::new()
    .add_identifier(MyTypes::Pe, PeIdentifier {})
    .add_analyzer(MyTypes::Pe, 0, PeHeaderAnalyzer {})
    .add_generic_analyzer(20, SignatureAnalyzer::from_file("rules.bin"))
    .add_extractor(MyTypes::Folder, FolderExtractor::<MyTypes>::new(true, false))
    .max_depth(16)
    .build();

for path in inputs {
    let mut content = FileContent::<MyTypes>::new(&path, false);
    let result = scanner.scan(&mut content, true);
    println!("{}: {} objects", path, result.objects_scanned());
    // copy anything you need to keep — see below
}
```

`scan` takes `&mut self`. One instance processes inputs **sequentially**, not in parallel. Share a scanner across threads only by giving each thread its own instance (build twice, or build once per worker).

## `ScanResult` borrows the scanner

The value `scan()` returns borrows the scanner’s `Context`. It stays valid **until the next `scan()` on the same instance**. Starting another scan clears that context; hanging on to the old `ScanResult` (or to strings and maps you borrowed from it) is use-after-clear.

Copy out what you must keep — counts, cloned header structs, owned finding text — before the next call. Walking the tree and printing in the same breath as `scan()` is fine.

Pools inside the context (path arena, local `VarMap`s) are reused across calls once they have grown, which is another reason not to throw the scanner away.

## What `scan` does

1. Clear the context.
2. If `filter_root` is `true` and a [filter](filter.md) is configured, test the root. A reject returns an empty `ScanResult` (no objects recorded).
3. Recursively identify, analyze, and extract, up to `max_depth` — [How one scan runs](how_one_scan_runs.md).

The second argument is **not** “enable the filter.” Children are always filtered (unless an `Entry` sets `skip_from_filtering`). It only decides whether the **root** is tested. Folders and ZIPs scanned with an extension filter almost always pass `false` — [Recursion and filter_root](recursion.md).

## What it holds

| Piece       | Role                                             |
| ----------- | ------------------------------------------------ |
| Identifiers | One per `ContentType`; compiled matchers         |
| Analyzers   | Typed lists + one generic list, by priority      |
| Extractors  | Per type, registration order                     |
| `Filter`    | Optional; root and/or children                   |
| `max_depth` | Recursion cap (default 8)                        |
| `Context`   | Cleared every `scan()`; borrowed as `ScanResult` |

You never construct a `Context`. Analyzers receive `&mut Context` during the scan; after `scan()` you read the same data through `ScanResult`.

## Chapter map

- [Builder](builder.md) — `new` / `with_metadata`, registration, `build()` panics.
- [Filter](filter.md) — include/exclude rules, precedence, default allow/deny.
- [Recursion and filter_root](recursion.md) — depth, the root flag, `skip_from_filtering`.
- [How one scan runs](how_one_scan_runs.md) — `inner_scan` / `extract_content`: `Skip`, `Exit`, requested extractors, `max_depth`.
