# Context and results

Analyzers do not `return` the interesting output of a scan. They write into a [`Context`](../chapter-3/scanner.md) the scanner threads through every `analyze` call. After `scan()` that same data is a [`ScanResult`](scan_result.md) you can read until the next scan on that instance.

You never construct a `Context`. It is owned by the [`Scanner`](../chapter-3/scanner.md) and **cleared at the start of every `scan()`** (object tree, maps, findings, extraction-request queue). Plugin instances, the [observer](../chapter-3/observer.md), and the [stop condition](../chapter-3/stop_condition.md) stay.

```rust
fn analyze(
    &mut self,
    content: &mut dyn Content<MyTypes>,
    context: &mut Context<MyTypes>,
) -> NextAction {
    context.global().set(var!("files"), 1u32);
    context.local().set(var!("size"), content.size());
    context.add_finding("ok", None, None);
    NextAction::Continue
}
```

There are four places an analyzer can write, on purpose:

| Channel | Lifetime | Typical use |
| --- | --- | --- |
| [`global`](global_vs_local.md) `VarMap` | One `scan()` call | Totals, flags, one value for the whole input |
| [`local`](global_vs_local.md) `VarMap` | One object | Headers, dimensions, a value you will look up on that node |
| [`add_finding`](findings.md) | Flat list on the scan | Detections: hashes, matches, labels |
| [`request_extract`](../chapter-2/requesting_extraction.md) | Queue for this object | Open extractors of another type on a byte window |

Maps and findings are **storage**. Extraction requests are **work for later in this object’s pipeline**. Small structs for the next analyzer stay in the context; payloads that should be their own nodes are children — [Extractions vs Context](../chapter-2/extractions_vs_context.md).

`context.objects_scanned()` is how many nodes are already in the tree (including the current one). Use it for progress or an analyzer-side budget (`NextAction::Exit` past a limit). A [stop condition](../chapter-3/stop_condition.md) is the same idea when the budget is not tied to `analyze`.

## `Context` versus `ScanResult`

During `analyze`, the context is **mutable**. After `scan()`, `ScanResult` is the **read-only** view of the same buffers:

- `res.global()` — the scan-wide map
- `res.local(handle)` — that object’s map, if one was allocated
- `res.findings()` — every stored finding, in emission order
- `res.root()` / `parent` / `child` / `next_sibling` — the [object tree](scan_result.md)

The result **borrows the scanner**. Starting another `scan()` on the same instance clears the context; hanging on to the old `ScanResult` (or to strings and maps you borrowed from it) is use-after-clear. Copy out what you must keep — counts, cloned structs, owned finding text — before the next call.

Pools (path arena, local `VarMap`s) are reused across calls once they have grown. That is another reason to [build once and scan many times](../chapter-3/scanner.md#create-once-scan-many-times).

## Chapter map

- [Global vs local VarMaps](global_vs_local.md) — when to use which, and the cost of a local map on every object.
- [Findings and metadata](findings.md) — `add_finding`, how to consume the list, `FindingMetadata`, `store_findings`.
- [Navigating ScanResult](scan_result.md) — parent / child / sibling, handles, walking the tree.
