# Observer

A `ScanObserver` watches a scan as it runs. It does not steer the pipeline and it does not replace [`ScanResult`](../chapter-4/scan_result.md). It is the live callback channel: progress, skips, extractions, findings, begin and end.

Attach one with [`ScannerBuilder::observer`](builder.md). Every method has an empty default, so an implementation only overrides the events it cares about. If you call `.observer(...)` twice, the second replaces the first.

```rust
struct Log;
impl ScanObserver<MyTypes> for Log {
    fn on_begin(&mut self, root: &str) {
        println!("begin {root}");
    }
    fn on_finding(&mut self, path: &str, finding: &str, _src: Option<&str>, _meta: Option<&NoMetadata>) {
        println!("{finding}  {path}");
    }
    fn on_end(&mut self) {
        println!("done");
    }
}

let mut scanner = ScannerBuilder::<MyTypes>::new()
    .observer(Log)
    .store_findings(false) // optional: stream findings, do not keep them
    // ...
    .build();
```

The observer is **owned by the scanner** and **survives `scan()`**. Clearing the [`Context`](../chapter-4/context.md) does not drop it. The same instance sees every later input on that scanner.

Callbacks run **synchronously** on the scan thread. Keep them cheap, or copy what you need out. Paths are the printable form from [`ContentPath::as_printable_string`](../chapter-2/content_path.md).

The type parameter `M` is the same [`FindingMetadata`](../chapter-4/findings.md) as the scanner. `ScannerBuilder::new()` gives `NoMetadata`; `with_metadata::<M>()` makes `on_finding` receive `Option<&M>`.

## Callbacks

| Method | When |
| --- | --- |
| `on_begin(root)` | Once at the start of `scan()`, **before** the root is filtered or identified. Runs even if the root is later rejected. |
| `on_filtered(path)` | The [filter](filter.md) rejected this path. The object is **not** scanned and is **not** in the result tree. |
| `on_scan_object(path, ty)` | After the object is identified and recorded, just before its analyzers run. `ty` is `None` when nothing identified it. Filtered-out content never reaches this callback. |
| `on_extraction(parent, entry)` | An extracted child **passed** the filter, just before the session materializes it. `entry` is the [`Entry`](../chapter-2/extractor.md) from `advance()`. |
| `on_finding(path, finding, source, metadata)` | An analyzer called `context.add_finding(...)`. Arguments match that call. |
| `on_end()` | Once when `scan()` returns, including when the root was filtered out or a [stop condition](stop_condition.md) aborted the walk. |

A directory walk that keeps `*.rs` looks like: `on_begin` → `on_scan_object` on the folder → `on_extraction` for each kept file (or `on_filtered` for the rest) → `on_scan_object` / `on_finding` on those files → `on_end`.

`on_extraction` is **not** called for a child the filter dropped. That child is `on_filtered` only. Folders that set `skip_from_filtering` still go through `on_extraction`.

## Findings without storing them

`add_finding` always notifies the observer. [`store_findings`](builder.md) (default `true`) only controls whether the hit is kept for `ScanResult::findings()`.

Set `.store_findings(false)` when the observer is the consumer — a live log, a UI, a counter — and you do not want the path arena to hold every digest or match until `scan()` returns. The `sha1` and `finder` examples in the repo do exactly that.

Streaming does not change analyzers: they still call `add_finding`. They do not need to know whether the result will retain the list.

## Timing a scan

`on_begin` / `on_end` are the right place for a wall-clock timer. Store `Instant::now()` on the observer in `on_begin`; print the elapsed duration in `on_end`. That interval includes filtering, identification, analysis, and extraction of the current `scan()` only — not builder time, and not the next input if you reuse the scanner.

## What an observer is not

It cannot `Skip` or `Exit`. Steering stays with analyzers (`NextAction`) and with [`StopCondition`](stop_condition.md). It cannot rewrite the filter. It does not see bytes, only paths and the finding strings analyzers already produced.
