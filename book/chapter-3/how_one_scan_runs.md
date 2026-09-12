# How one scan runs

[Architecture](../chapter-2/architecture.md) is the map: one object through filter → type → record → analyze → extract, then each child around the same loop. This page is the control flow inside one `scan()` call — what `inner_scan`, `scan_object`, and `extract_content` actually do with `Skip`, `Exit`, `ReprocessAsType`, requested extractors, `max_depth`, `max_change_type`, the [observer](observer.md), and a [stop condition](stop_condition.md).

## `scan()`

```rust
let result = scanner.scan(&mut content, filter_root);
```

1. **Clear** the scanner’s `Context`: object tree, maps, findings, extraction-request queue. Plugin instances stay. So do the observer and stop condition.
2. **`on_begin(root)`** if an [observer](observer.md) is attached.
3. If `filter_root` is `true` and a [filter](filter.md) is configured, test the root. A reject calls `on_filtered` then `on_end` and returns an empty `ScanResult` — no object is recorded.
4. Call `inner_scan` on the root at **depth 1**, with no parent.
5. **`on_end()`**, then return a `ScanResult` that borrows the context until the next `scan()`.

The filter on the root is this special case only. Children are tested later, inside extraction, unless their `Entry` sets `skip_from_filtering`.

## `inner_scan` (one object)

Every object that is actually visited — the accepted root, or a child that passed the filter — runs this function.

1. Snapshot the extraction-request stack length for this object.
2. If a [stop condition](stop_condition.md) is attached and `should_stop()` is `true`, return `Exit` **without** recording this object.
3. **Identify** and **record** the object (`create_object`). The node is in the tree *before* any analyzer runs, even if the type is unknown.
4. Call `scan_object` (below).
5. Restore the request stack (release unused params, truncate to the snapshot).
6. Return to the caller:
   - `Continue` or `Skip` from this object both become **`Continue`** for the parent session.
   - `Exit` stays **`Exit`**.
   - A child’s `ReprocessAsType` is handled **inside** that child’s `scan_object`; the parent session only sees `Continue` or `Exit`.

That mapping is the whole difference between “stop this object” and “stop the scan.” A child’s `Skip` never ends the parent’s extraction session. A child’s `Exit` does.

## `scan_object` (same node)

`scan_object` is the analyze / extract pass. `ReprocessAsType` re-enters it on the **same** tree node; it does not go back through the filter, the stop condition, or identification.

1. **`on_scan_object(path, ty)`.** Fires again after each accepted type change.
2. **Analyze.** Typed analyzers for the current type, then generics. The first `Skip`, `Exit`, or `ReprocessAsType` from a **typed** analyzer stops the rest of this pass (including generics). A generic analyzer that returns `ReprocessAsType` is currently treated as `Continue`. Each `add_finding` notifies `on_finding`.
3. Match the outcome:
   - `Continue` → `run_extractors(All)`: own-type extractors, then requested extractors.
   - `Skip` → no extractors; return `Continue` to the parent.
   - `Exit` → return `Exit`.
   - `ReprocessAsType(new_ty)` → `run_extractors(OnlyRequested)`, restore the request stack, then if `change_type_count < max_change_type` call `Content::set_content_type`, update the node’s `type_id`, and re-enter `scan_object`. Over the cap, return `Continue` and keep the current type.

`max_change_type` defaults to **2** and is clamped to `1..=T::COUNT`. Identifiers are not re-run.

## Identification

`Content::content_type()` already `Some(ty)` skips the identifier table.

Otherwise the scanner computes three candidates — file name, extension, magic — then confirms them with `validate` in this order: **magic → name → extension**. Identifiers with no `IdentifyMethod` run last. The first 16 bytes are read only if at least one magic identifier is registered.

At most one identifier exists per variant, so a successful `validate` names a single type. Failure at every candidate leaves the object unidentified: generics still run; typed analyzers and own-type extractors do not. Detail: [Identifier](../chapter-2/identifier.md).

## Extraction sessions

`run_extractors` has two modes, both on the **current** object’s bytes:

- **`All`** (`Continue`): extractors registered for the object’s **own** type, in registration order, then requested extractors.
- **`OnlyRequested`** (`ReprocessAsType`): requested extractors only. Own-type extractors for the *old* type do not run.

For each [extraction request](../chapter-2/requesting_extraction.md) queued while this object was analyzed, extractors registered for the **requested** type run on the requested window (`start` / `len` / params). A request whose type has no extractor is dropped.

Each extractor `create_session`s, then `advance` / `extract` in a loop (`extract_content`):

- If `depth >= max_depth`, extraction does not start. The parent is already recorded and analyzed; no children.
- An `Entry` that fails the filter (and is not `skip_from_filtering`) is skipped — `on_filtered`, no object, next entry.
- An `Entry` that passes the filter is announced with `on_extraction`, then materialized.
- A yielded child is a new `inner_scan` at `depth + 1`, parented to this object.
- Child returns `Continue` (including a child that `Skip`ped itself) → next entry in **this** session.
- Child returns `Exit` → this session is dropped, remaining extractors for this object are not started, and `Exit` unwinds to `scan()` (`on_end` still runs).

Objects already in the tree stay there. `Exit` does not roll back the context.

Extractors do not return `AnalysisOutcome`. They yield `Option`. Only analyzers steer the scan; the session loop only observes what `inner_scan` mapped `Skip` / `Exit` into (`ReprocessAsType` never leaves `scan_object`).

## What `Skip`, `Exit`, and `ReprocessAsType` mean in practice

| Analyzer on object *X* returns | Analyzers left on *X* | Own-type extractors | Requested extractors | Then | Parent session | Rest of the scan |
| ------------------------------ | --------------------- | ------------------- | -------------------- | ---- | -------------- | ---------------- |
| `Continue`                     | run                   | run                 | run                  | done | continues      | continues        |
| `Skip`                         | stop                  | **do not run**      | **do not run**       | done | **continues** (next sibling) | continues |
| `Exit`                         | stop                  | **do not run**      | **do not run**       | unwind | **dropped**  | **stops**        |
| `ReprocessAsType(ty)`          | stop this pass        | **do not run**      | **run**              | set type, re-enter `scan_object` if under `max_change_type` | continues (unless a child `Exit`s) | continues |

`Skip` is “this object is done; keep walking.” `Exit` is “unwind.” `ReprocessAsType` is “finish queued extractions, then analyze this same node as another type.” All three leave *X* in the result tree with whatever analyzers wrote before they returned. After a successful reprocess, *X*’s `type_id` is the new type.

A [stop condition](stop_condition.md) that fires at the start of *X* is the same unwind as `Exit`, except *X* is never identified or recorded. `on_end` still runs.

## Putting it together

A folder whose extractor yields `a.bin`, `b.bin`, `c.bin`:

- An analyzer on `b.bin` returns `Skip` → `b.bin` is not extracted further; `c.bin` is still visited.
- An analyzer on `b.bin` returns `Exit` → `c.bin` is never opened; `scan()` returns with `a.bin` and `b.bin` already recorded.
- An analyzer on `b.bin` returns `ReprocessAsType` → requested extractors on `b.bin` run first; then `b.bin` is re-analyzed as the new type (same tree node). `c.bin` is still visited unless a nested `Exit` unwinds the folder session.
- A [stop condition](stop_condition.md) that fires as `c.bin` is about to start → `c.bin` is never recorded; `a.bin` and `b.bin` stay in the result.

An embedded ZIP requested from a PE analyzer runs after the PE’s own extractors (`Continue`) or *instead of* them (`ReprocessAsType`). Those ZIP members are ordinary children of the PE object: same `inner_scan`, same filter, same depth check.

Reuse, filters, observers, stop conditions, and depth knobs stay in this chapter: [Scanner](scanner.md), [Filter](filter.md), [Observer](observer.md), [Stop condition](stop_condition.md), [Recursion and filter_root](recursion.md). Where plugins write (`VarMap`, findings, the result tree) is [Chapter 4](../chapter-4/context.md).
