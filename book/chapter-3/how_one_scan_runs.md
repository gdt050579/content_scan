# How one scan runs

[Architecture](../chapter-2/architecture.md) is the map: one object through filter → type → record → analyze → extract, then each child around the same loop. This page is the control flow inside one `scan()` call — what `inner_scan` and `extract_content` actually do with `Skip`, `Exit`, requested extractors, and `max_depth`.

## `scan()`

```rust
let result = scanner.scan(&mut content, filter_root);
```

1. **Clear** the scanner’s `Context`: object tree, maps, findings, extraction-request queue. Plugin instances stay.
2. If `filter_root` is `true` and a [filter](filter.md) is configured, test the root. A reject returns an empty `ScanResult` — no object is recorded.
3. Call `inner_scan` on the root at **depth 1**, with no parent.
4. Return a `ScanResult` that borrows the context until the next `scan()`.

The filter on the root is this special case only. Children are tested later, inside extraction, unless their `Entry` sets `skip_from_filtering`.

## `inner_scan` (one object)

Every object that is actually visited — the accepted root, or a child that passed the filter — runs this function.

1. Snapshot the extraction-request stack length for this object.
2. **Identify** and **record** the object (`create_object`). The node is in the tree *before* any analyzer runs, even if the type is unknown.
3. **Analyze.** Typed analyzers for the resolved type, then generics. The first `Skip` or `Exit` stops the rest of this object’s analyzers.
4. **Extract** only if analysis returned `Continue`. Own-type extractors, then requested extractors (below).
5. Restore the request stack (release unused params, truncate to the snapshot).
6. Return to the caller:
   - `Continue` or `Skip` from this object both become **`Continue`** for the parent session.
   - `Exit` stays **`Exit`**.

That mapping is the whole difference between “stop this object” and “stop the scan.” A child’s `Skip` never ends the parent’s extraction session. A child’s `Exit` does.

## Identification

`Content::content_type()` already `Some(ty)` skips the identifier table.

Otherwise the scanner computes three candidates — file name, extension, magic — then confirms them with `validate` in this order: **magic → name → extension**. Identifiers with no `IdentifyMethod` run last. The first 16 bytes are read only if at least one magic identifier is registered.

At most one identifier exists per variant, so a successful `validate` names a single type. Failure at every candidate leaves the object unidentified: generics still run; typed analyzers and own-type extractors do not. Detail: [Identifier](../chapter-2/identifier.md).

## Extraction sessions

`run_extractors` does two passes, both on the **current** object’s bytes:

1. Extractors registered for the object’s **own** type, in registration order.
2. For each [extraction request](../chapter-2/requesting_extraction.md) queued while this object was analyzed, extractors registered for the **requested** type, on the requested window (`start` / `len` / params). A request whose type has no extractor is dropped.

Each extractor `create_session`s, then `advance` / `extract` in a loop (`extract_content`):

- If `depth >= max_depth`, extraction does not start. The parent is already recorded and analyzed; no children.
- An `Entry` that fails the filter (and is not `skip_from_filtering`) is skipped — no object, next entry.
- A yielded child is a new `inner_scan` at `depth + 1`, parented to this object.
- Child returns `Continue` (including a child that `Skip`ped itself) → next entry in **this** session.
- Child returns `Exit` → this session is dropped, remaining extractors for this object are not started, and `Exit` unwinds to `scan()`.

Objects already in the tree stay there. `Exit` does not roll back the context.

Extractors do not return `NextAction`. They yield `Option`. Only analyzers steer the scan; the session loop only observes what `inner_scan` mapped `Skip` / `Exit` into.

## What `Skip` and `Exit` mean in practice

| Analyzer on object *X* returns | Analyzers left on *X* | Extractors on *X* | Parent session               | Rest of the scan |
| ------------------------------ | --------------------- | ----------------- | ---------------------------- | ---------------- |
| `Continue`                     | run                   | run               | continues                    | continues        |
| `Skip`                         | stop                  | **do not run**    | **continues** (next sibling) | continues        |
| `Exit`                         | stop                  | **do not run**    | **dropped**                  | **stops**        |

`Skip` is “this object is done; keep walking.” `Exit` is “unwind.” Both leave *X* in the result tree with whatever analyzers wrote before they returned.

## Putting it together

A folder whose extractor yields `a.bin`, `b.bin`, `c.bin`:

- An analyzer on `b.bin` returns `Skip` → `b.bin` is not extracted further; `c.bin` is still visited.
- An analyzer on `b.bin` returns `Exit` → `c.bin` is never opened; `scan()` returns with `a.bin` and `b.bin` already recorded.

An embedded ZIP requested from a PE analyzer runs after the PE’s own extractors. Those ZIP members are ordinary children of the PE object: same `inner_scan`, same filter, same depth check.

Reuse, filters, and depth knobs stay in this chapter: [Scanner](scanner.md), [Filter](filter.md), [Recursion and filter_root](recursion.md). Where plugins write (`VarMap`, findings, the result tree) is [Chapter 4](../chapter-4/context.md).
