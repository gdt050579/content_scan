# Global vs local VarMaps

Both maps are [`varmap`](https://crates.io/crates/varmap) `VarMap`s, re-exported by `content_scan`. Keys are compile-time typed (`var!("name")`). Values are primitives or types that derive `VarMapValue` (typically `Copy + Clone`).

They differ in **scope**, not in API.

```rust
context.global().set(var!("sum"), value);
context.local().set(var!("value"), value);

let sum = res.global().get::<u32>(var!("sum")).unwrap_or(0);
let v = res.local(handle).and_then(|m| m.get::<u32>(var!("value")));
```

## When to use global

**One value for the whole `scan()` call.** Anything that should accumulate across objects, or that you will read once at the end without walking the tree.

- Totals: vowel count, byte sum, number of PNGs
- Scan-wide flags: “saw a packed file”, “hit the size cap”
- A single report line you would print even if you never look at children

The `sum` example adds each extracted number into `var!("sum")` on the global map, and also stores that number on the child — global for the headline, local for the tree dump.

```rust
if !context.global().update(var!("sum"), |v: &mut u32| *v += value) {
    context.global().set(var!("sum"), value);
}
```

`update` fails when the key is missing; `set` seeds it. There is **one** global map per scan. It is cheap. Clearing it at the next `scan()` is the whole lifetime.

Do **not** put per-file hashes or per-file sizes in global under a single key: the next object overwrites them. Do **not** put signature databases in global either — the context is cleared every scan; load tables on the [analyzer struct](../chapter-2/analyzer.md#loading-data-at-builder-time).

## When to use local

**A value that belongs to this object**, either because a later analyzer on the **same** object will read it, or because after the scan you will look it up on that node while walking the tree.

- PE headers for the icon analyzer on this PE ([blackboard](../chapter-2/analyzer.md#writing-the-context-for-another-analyzer))
- `{width, height}` on this PNG (`image_size`)
- The numeric value of this extracted `Number` child (`sum`)

Later analyzers on the same object see the same map: `context.local()` is created **lazily on the first call** for the current object and reused until that object is finished. Priority / `requires` decide who writes first.

After `scan()`, `res.local(handle)` returns `Some(&VarMap)` only if **some** analyzer actually called `context.local()` on that object. Otherwise it is `None` — not an empty map, **no map**.

Local is the wrong place for:

- A running total (that is global)
- A detection you want as a flat list (that is a [finding](findings.md))
- A decoded file body (that is an [extraction](../chapter-2/extractions_vs_context.md))

## Memory: a local map is not free

Every **visited** object is a node in the result tree (path, type, parent/child/sibling links). That cost you pay anyway.

A **local `VarMap` is extra**. It is taken from an internal pool and attached to the object the first time `context.local()` runs. If no analyzer on that object ever calls `local()`, **no map is allocated for it**.

A generic analyzer that always does this:

```rust
context.local().set(var!("hash"), hash);
```

…creates one map **per file**. On a directory of two million objects that is two million maps held until `scan()` returns, plus whatever you stored in them. The pool reuses maps across **later** scans (`KeepSmallestN` on clear); it does not shrink the current scan.

Prefer:

| You need | Put it |
| --- | --- |
| One number at the end | `global` |
| A list of hits to print or ship | [`add_finding`](findings.md) (or an [observer](../chapter-3/observer.md) and `store_findings(false)`) |
| A struct another analyzer on **this** file will read | `local` |
| Width/height you will print **while walking that node** | `local` |
| Nothing about this file except that it was visited | neither map — the tree node is enough |

Hashes in the md5 example are findings, not local keys: you iterate `res.findings()`, you do not allocate a map per file. Image sizes are local because the `image_size` dump prints `size` **on that handle** as it walks.

If you only need the hash in a log line as the scan runs, skip both maps: `add_finding` plus `store_findings(false)` and an observer. Then you do not retain the text in the arena until `scan()` returns either.

## Custom values

```rust
#[derive(Debug, Copy, Clone, Eq, PartialEq, VarMapValue)]
struct Size {
    width: u32,
    height: u32,
}

context.local().set(var!("size"), Size { width, height });
```

`VarMapValue` is re-exported from `content_scan`. Keep stored types small and `Copy`. A `Vec<u8>` of the file body does not belong here.

## After the scan

```rust
let files = res.global().get::<u32>(var!("files")).unwrap_or(0);

if let Some(root) = res.root() {
    if let Some(map) = res.local(root) {
        let size = map.get::<Size>(var!("size"));
    }
}
```

`res.global()` is always there (possibly empty). `res.local(handle)` is `None` for objects that never touched `context.local()`. Walk the tree as in [Navigating ScanResult](scan_result.md).
