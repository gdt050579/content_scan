# Architecture

The scanner is a dispatcher. It does not know what a PNG is, or how to unpack a ZIP. It knows *when* to call your plugins, *in what order*, and *where those plugins are allowed to leave results*.

This page is the map. [How one scan runs](../chapter-3/how_one_scan_runs.md) is that loop as `inner_scan` and `extract_content` implement it — `Skip` versus nested sessions, `Exit`, requested extractors, `max_depth`, observer callbacks, and the stop-condition check.

## The assembled scanner

A `Scanner` is built once, then reused for many `scan()` calls. It holds:

- the optional [`Filter`](../chapter-3/filter.md)
- the identifier table (one slot per `ContentType`)
- the analyzer list (typed and generic)
- the extractor list (typed only)
- a `max_depth`
- an optional [`ScanObserver`](../chapter-3/observer.md) and [`StopCondition`](../chapter-3/stop_condition.md)
- a [`Context`](../chapter-4/context.md) that is **cleared at the start of every scan** and filled as plugins run

You never construct a `Context` yourself. Analyzers receive `&mut Context` during the scan. After `scan()` returns, the same data is exposed as a `ScanResult` that borrows from the scanner until the next scan.

```text
                         ┌───────────────────────────────────┐
                         │             Scanner               │
            ┌──────┐     │                                   │
   Content ─┤scan()├───► │  Filter?                          │
            └──────┘     │  Identifiers  (1 per type)        │
                         │  Analyzers    (N, typed+generic)  │
                         │  Extractors   (N, per type)       │
                         │  max_depth                        │
                         │  Observer?  StopCondition?        │
                         │                                   │
                         │          ┌──────────────┐         │
                         │          │   Context    │         │
                         │          │  global map  │         │
                         │          │  object tree │         │
                         │          │  local maps  │         │
                         │          │  findings    │         │
                         │          └──────┬───────┘         │
                         └─────────────────┼─────────────────┘
                                           │
                                           ▼
                                       ScanResult
                                  (borrows the Context)
```

Plugins **read** `Content`. They **write** the `Context`. The only results of a scan that outlive the call as `ScanResult` are whatever those plugins stored there — variable maps, findings, and the tree of objects the scanner recorded while it walked. An [observer](../chapter-3/observer.md) can watch the same events live without being part of that result.

## Plugin cardinality

This is the rule that shapes every scanner you will build:

| Plugin | How many per `ContentType` | Order |
| --- | --- | --- |
| Identifier | **Exactly one** (or none). A second registration for the same type panics in `build()`. | Not sequenced against each other: they compete as candidates during type resolution. |
| Analyzer | **Any number**, including several for the same type. Plus a separate **generic** bucket that runs on every object. | Ascending `priority` (lower first). Typed analyzers for the resolved type, then generics. |
| Extractor | **Any number**, including several for the same type. There is no generic extractor. | Registration order, for the resolved type, then for each type an analyzer [requested](requesting_extraction.md). |

Identifiers are one-per-type because type resolution must produce a **single** `ContentType` (or none). Analyzers and extractors are lists because a PNG might need a dimension reader *and* a checksum, and a ZIP might unpack members *and* look for an embedded extra stream.

A realistic registration looks like this — two types, several analyzers (two of them for `Png`), two extractors for `Zip`, and one identifier each:

```rust
let scanner = ScannerBuilder::<MyTypes>::new()
    // one identifier per type — a second add_identifier(MyTypes::Png, ...)
    // would panic in build()
    .add_identifier(MyTypes::Png, PngIdentifier {})
    .add_identifier(MyTypes::Zip, ZipIdentifier::new())
    // several analyzers for the same type, ordered by priority
    .add_analyzer(MyTypes::Png, 0, PngSizeAnalyzer {})
    .add_analyzer(MyTypes::Png, 10, PngChecksumAnalyzer {})
    .add_analyzer(MyTypes::Zip, 0, ZipPrinter {})
    // generic: runs on every object, after the typed analyzers
    .add_generic_analyzer(0, HashAnalyzer {})
    // several extractors for the same type, in registration order
    .add_extractor(MyTypes::Zip, ZipExtractor::new())
    .add_extractor(MyTypes::Zip, ZipExtraStreamExtractor {})
    .add_extractor(MyTypes::Folder, FolderExtractor::<MyTypes>::new(true, false))
    .build();
```

If two typed analyzers share the same `(content_type, priority)`, both run; their relative order is unspecified. Extractors have no priority byte: swap the two `.add_extractor(MyTypes::Zip, …)` lines and they swap execution order.

## Pipeline for one object

Every object — the root you passed to `scan()`, or a child an extractor just produced — goes through the same steps. Children start a fresh pass at `depth + 1`.

```text
                         ┌─────────────┐
                         │   Content   │
                         └──────┬──────┘
                                │
                                ▼
                         ┌─────────────┐
        dropped ◄────────│   Filter?   │◄───────────────────────────────────────┐
                         └──────┬──────┘                                        │
                             accept                                             │
                                │                                               │
                                ▼                                               │
                   ┌────────────────────────┐  yes                              │
                   │ is content_type() set? │───────┐                           │
                   └────────────┬───────────┘       │                           │
                                │ no                │                           │
                                ▼                   │                           │
              ┌───────────────────────────┐         │                           │
              │ Identifiers (1 per type)  │         │                           │
              │ magic → name → extension  │         │                           │
              │ → custom validate         │         │                           │
              └─────────────┬─────────────┘         │                           │
                            └───────────────┬───────┘                           │
                                            │                                   │
                                            ▼                                   │
                              ┌───────────────────────────┐                     │
                              │ Record object in Context  │                     │
                              └─────────────┬─────────────┘                     │
                                            │                                   │
                                            ▼                                   │
                              ┌───────────────────────────┐                     │
                              │ Type-specific analyzers   │──┐                  │
                              │ 0..N for this type        │  │                  │
                              │ (by priority)             │  │                  │
                              └─────────────┬─────────────┘  │                  │
                                            │                │ write            │
                                            ▼                ├──────► Context   │
                              ┌───────────────────────────┐  │        maps +    │
                              │ Generic analyzers         │──┘        findings  │
                              │ 0..N (by priority)        │                     │
                              └─────────────┬─────────────┘                     │
                                            │                                   │
                                            ▼                                   │
                                       NextAction                               │
                                  ┌─────────┼─────────┐                         │
                               Skip       Exit     Continue                     │
                                  │         │         │                         │
                                  ▼         ▼         ▼                         │
                               stop      abort   ┌───────────────────────────┐  │
                               this      entire  │ Extractors for this type  │  │
                               object    scan    │ 0..N (registration order) │  │
                                                 └─────────────┬─────────────┘  │
                                                               │                │
                                                               ▼                │
                                                 ┌───────────────────────────┐  │
                                                 │ Requested extractors      │  │
                                                 │ 0..N per request          │  │
                                                 └─────────────┬─────────────┘  │
                                                               │                │
                                                               ▼                │
                                                 ┌───────────────────────────┐  │
                                                 │ Each child Content        ├──┘
                                                 │ if depth < max_depth      │
                                                 └───────────────────────────┘
```

In words:

1. **Filter.** If a filter is configured and this object is subject to it, a reject means the object is not scanned at all (`on_filtered` if an observer is attached). The root is tested only when `scan(..., filter_root)` is `true`. Extracted children are tested unless their `Entry` sets `skip_from_filtering`. See [Recursion and filter_root](../chapter-3/recursion.md).
2. **Stop condition.** At the start of `inner_scan`, before identification, an optional [`StopCondition`](../chapter-3/stop_condition.md) can abort the whole scan. That object is not recorded.
3. **Type.** If `Content::content_type()` already returns `Some(ty)`, identifiers are skipped. Otherwise the identifier table proposes candidates — magic (first 16 bytes), then file name, then extension, then identifiers with no `IdentifyMethod` — and each candidate’s `validate` must accept. At most one identifier exists for each variant, so a match names a type unambiguously.
4. **Record.** The scanner appends the object to the context’s tree (path, resolved type, parent/child/sibling links) *before* analyzers run, then `on_scan_object`.
5. **Analyze.** All analyzers registered for that type run, lowest priority first. Then all generic analyzers run, again by priority. Unidentified objects still get the generic bucket. Each analyzer returns a `NextAction`: `Continue`, `Skip` (no further analyzers or extractors on **this** object), or `Exit` (unwind the whole scan). Findings notify `on_finding`.
6. **Extract.** If analysis continued, extractors registered for the object’s own type run, then extractors for any type an analyzer requested with `request_extract`. Each extractor opens a session and yields children (`on_extraction` after the filter); each child goes back to step 1 at the next depth, until `max_depth`.

Extractors and sessions do not return `NextAction`. They yield `Option`. Only analyzers steer the scan.

The same object never runs “some other type’s” typed analyzers. A file identified as `Png` runs `Png` analyzers (and generics), not `Zip` analyzers. It *can* still run `Zip` **extractors** if an analyzer requested `Zip` on a byte range — that is how embedded archives are opened without re-typing the parent. [Requesting extraction](requesting_extraction.md) covers that mechanism.

## Where data is stored

Analyzers do not `return` the interesting output of a scan. They write into the `Context` that the scanner threads through every `analyze` call. After `scan()` that context is what you read as `ScanResult`.

There are two places plugins put information, on purpose:

**The context maps.** A **global** `VarMap` lives for the whole `scan()` call: totals, flags, anything that should accumulate across objects. A **local** `VarMap` is attached to the current object: width and height of *this* PNG, the value of *this* extracted number. After the scan you look up the global map on the result, and each object’s local map through the result tree.

**Findings.** A flat list of hits recorded with `context.add_finding(...)`. Each finding belongs to the object that was current when it was emitted. Findings are the “something was detected here” channel — a hash, an entropy label, a YARA-like match — as opposed to structured fields you intend to query on a specific node. An [observer](../chapter-3/observer.md) is notified on each `add_finding` even when the scanner is told not to keep the list (`store_findings(false)`).

```text
  Analyzer::analyze(content, context)
        │
        ├── context.global().set(...)     ─┐
        ├── context.local().set(...)      ─┼── Context
        ├── context.add_finding(...)      ─┤     └── findings[]
        └── context.request_extract(...)  ─┘         (queue for step 6)
```

Treat both as a **general notion** for now: maps are how you stash typed values; findings are how you emit a list of detections. The APIs (`var!`, `VarMapValue`, finding metadata, walking `ScanContentHandle`s, the lifetime of `ScanResult`) are [Chapter 4](../chapter-4/context.md).

The scanner itself also writes the context: every visited object is a node in the tree, even if no analyzer stored anything on it. That is why `objects_scanned()` and parent/child navigation exist independently of your plugins.

## What this picture leaves out

The architecture is complete enough to read the rest of the book against:

- **One identifier per type; many analyzers and extractors, including several for the same type.**
- **Typed analyzers then generic analyzers; typed extractors then requested extractors.**
- **Results live in the context (maps + object tree) and in findings.** Chapter 4 is where those structures are defined.

Not yet: `IdentifyMethod` variants and the 16-byte magic window ([Identifier](identifier.md)), analyzer `Dependencies` and `NextAction` ([Analyzer](analyzer.md)), sessions / `OwnedContentPtr` / `Entry` ([Extractor](extractor.md)), builder panics and `with_metadata` ([Builder](../chapter-3/builder.md)), [observer](../chapter-3/observer.md) and [stop condition](../chapter-3/stop_condition.md), or the exact `Skip`/`Exit` interaction with nested sessions ([How one scan runs](../chapter-3/how_one_scan_runs.md)).
