# Stop condition

A `StopCondition` aborts a scan from **outside** the analyzer loop: a timeout, a cancellation flag, a global hit cap. Attach one with [`ScannerBuilder::stop_condition`](builder.md). If you call it twice, the second replaces the first.

```rust
pub trait StopCondition {
    fn should_stop(&mut self) -> bool {
        false
    }
}
```

The default never stops. `should_stop` is checked at the **start of every content object**, in `inner_scan`, **before** identification and analysis. When it returns `true`, that object is **not** recorded. The scanner unwinds with `NextAction::Exit` and `scan()` returns the [`ScanResult`](../chapter-4/scan_result.md) accumulated so far. Objects already in the tree stay there.

```rust
struct Deadline(std::time::Instant);
impl StopCondition for Deadline {
    fn should_stop(&mut self) -> bool {
        std::time::Instant::now() >= self.0
    }
}

let mut scanner = ScannerBuilder::<MyTypes>::new()
    .stop_condition(Deadline(std::time::Instant::now() + std::time::Duration::from_secs(30)))
    // ...
    .build();
```

An [`observer`](observer.md) still gets `on_end` after an abort. It does **not** get `on_scan_object` for the object that was about to start.

## Versus `NextAction::Exit`

Analyzers can already abort with [`NextAction::Exit`](../chapter-2/analyzer.md#nextaction). Use that when **the plugin** decides the scan is done (signature hit, corrupt container, budget it tracks in the [`Context`](../chapter-4/context.md)).

Use a stop condition when the decision is **not** tied to the current file’s analysis:

- a wall-clock deadline
- an `Arc<AtomicBool>` flipped by Ctrl+C or a UI cancel button
- “stop after *N* objects” counted in the condition itself

| | Analyzer `Exit` | `StopCondition` |
| --- | --- | --- |
| When | During `analyze` on an object that is already in the tree | Before that object is identified |
| Current object | Recorded; some analyzers may have run | **Not** recorded |
| Typical owner | The plugin | The application around the scanner |

Both leave prior objects in the result. Neither rolls the context back.

Filtered-out children never reach `inner_scan`, so `should_stop` is **not** called for them. A huge directory with a tight extension filter only checks the condition on files the filter kept (and on folders, which skip the filter).

## Lifetime on a reused scanner

The condition is **owned by the scanner** and is **not** reset when `scan()` clears the context. A counter or a deadline you moved in at `build()` keeps ticking across inputs.

That is what you want for a process-wide cancel flag. It is not what you want for “thirty seconds **per file**” unless you share state you can refresh:

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

struct Cancel(Arc<AtomicBool>);
impl StopCondition for Cancel {
    fn should_stop(&mut self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

let flag = Arc::new(AtomicBool::new(false));
let mut scanner = ScannerBuilder::<MyTypes>::new()
    .stop_condition(Cancel(flag.clone()))
    .build();

// later, from another thread or a signal handler:
flag.store(true, Ordering::Relaxed);
```

There is no API to replace the condition after `build()`. Plan the sharing (or rebuild the scanner) when the budget is per `scan()` rather than per process.

## Where it sits in the loop

[How one scan runs](how_one_scan_runs.md) is the full control flow. In short: `on_begin` → optional root filter → `inner_scan` (stop check → identify → analyze → extract) → `on_end`. Hitting the stop check is the same unwind as an analyzer `Exit`, minus the current object.
