# Navigating ScanResult

Every object the scanner **actually visits** becomes a node: interned printable path, resolved [`ContentType`](../chapter-2/content_type.md) (or none), optional [local map](global_vs_local.md), and links to its family. The links are a **parent / first-child / next-sibling** tree that mirrors **extraction order**.

You never see raw indices. Navigation uses an opaque, `Copy` [`ScanContentHandle`](context.md). A handle is only valid for the `ScanResult` it came from, and only until the next `scan()` on that scanner.

```rust
impl<'a, T: ContentType, M: FindingMetadata> ScanResult<'a, T, M> {
    pub fn global(&self) -> &VarMap;
    pub fn objects_scanned(&self) -> u32;
    pub fn findings(&self) -> FindigsIterator<'a, T, M>;

    pub fn root(&self) -> Option<ScanContentHandle>;
    pub fn parent(&self, handle: ScanContentHandle) -> Option<ScanContentHandle>;
    pub fn child(&self, handle: ScanContentHandle) -> Option<ScanContentHandle>;
    pub fn next_sibling(&self, handle: ScanContentHandle) -> Option<ScanContentHandle>;

    pub fn path(&self, handle: ScanContentHandle) -> Option<&str>;
    pub fn content_type(&self, handle: ScanContentHandle) -> Option<T>;
    pub fn local(&self, handle: ScanContentHandle) -> Option<&VarMap>;
}
```

`path` is `ContentPath::as_printable_string()` interned at record time. `content_type` is `None` when identification failed (generics still ran). `local` is `None` when no analyzer called `context.local()` on that object.

## What is in the tree (and what is not)

**In:** the accepted root; every extracted child that passed the [filter](../chapter-3/filter.md) (or set `skip_from_filtering`); unidentified files; folders; ZIP members that were materialized.

**Not in:** a root rejected with `filter_root = true`; an `Entry` the filter dropped (`on_filtered` only); a child never `extract()`ed; objects skipped because `max_depth` forbade starting extraction (the **parent** is in the tree; the would-be children are not).

An analyzer `Skip` leaves **that** object in the tree; it just has no further analyzers or children from its own extractors. An analyzer `Exit` or a [stop condition](../chapter-3/stop_condition.md) leaves everything already recorded; the object that did not start is absent.

`objects_scanned()` is the number of nodes, including the root. An empty result (`root() == None`) usually means the root was filtered out.

## Relations

Each node stores:

- **parent** — the object whose extractor yielded this one. The root has none.
- **first child** — the first yielded child that was actually scanned, in extraction order.
- **next sibling** — the next child of the **same parent**, later in that same yield order.

There is no public previous-sibling, last-child, or “nth child.” Children are a **singly linked list** headed at `child(parent)`.

```text
  folder/                  root
  ├── a.png                child(folder)
  ├── sub/                 next_sibling(a.png)
  │   └── b.png            child(sub)
  └── c.jpg                next_sibling(sub)
```

| From | Call | Result |
| --- | --- | --- |
| `folder` | `parent` | `None` |
| `folder` | `child` | `a.png` |
| `a.png` | `parent` | `folder` |
| `a.png` | `next_sibling` | `sub` |
| `a.png` | `child` | `None` (no extracted children) |
| `sub` | `child` | `b.png` |
| `sub` | `next_sibling` | `c.jpg` |
| `b.png` | `parent` | `sub` |
| `b.png` | `next_sibling` | `None` |
| `c.jpg` | `next_sibling` | `None` |

Order is the extractor’s `advance` / `extract` order, then [requested extractors](../chapter-2/requesting_extraction.md) after own-type extractors. Nested ZIP members hang under the ZIP object, not under the folder that contained the ZIP.

Siblings share a parent. `next_sibling` is **not** “the next file on disk in a flat list”; after the last child of `sub` you do not automatically land on `c.jpg` — that is `next_sibling(sub)`, one level up.

## Walking children of one node

First child, then walk the sibling chain:

```rust
let mut c = res.child(parent);
while let Some(h) = c {
    // h is one child of parent
    c = res.next_sibling(h);
}
```

That is the whole child list. It does **not** descend. For a full dump, recurse:

```rust
fn dump<T: ContentType>(res: &ScanResult<T>, h: ScanContentHandle, depth: usize) {
    let pad = "  ".repeat(depth);
    let path = res.path(h).unwrap_or("?");
    let ty = res.content_type(h);
    println!("{pad}- {path} ({ty:?})");

    let mut c = res.child(h);
    while let Some(cur) = c {
        dump(res, cur, depth + 1);
        c = res.next_sibling(cur);
    }
}

if let Some(root) = res.root() {
    dump(&res, root, 0);
}
```

That is **preorder DFS**: visit the node, then its children left to right, each child’s subtree before the next sibling. It matches how extraction nested.

An equivalent shape (used by `image_size`) visits `handle`, recurses into `child(handle)` at `depth + 1`, then recurses into `next_sibling(handle)` at the **same** depth. Starting from `root()` those two walks print the same tree. Starting from an interior node, the sibling form would also print later siblings of that node — the child-loop form above will not.

## Looking up data on a handle

```rust
let path = res.path(h).unwrap_or("?");
let ty = res.content_type(h); // Option<MyTypes>
let local = res.local(h);     // Option<&VarMap>
```

Combine with [findings](findings.md): the flat list is not a child walk. If you need “hits on this file, then its children,” either:

- iterate `res.findings()` and group by `f.path()`, or
- walk the tree and ignore the finding list (when the interesting data is in `local`), or
- stream with an observer during the scan.

There is no `findings_for(handle)` helper.

## Lifetime

`ScanResult` borrows the scanner. Handles are indices into that scan’s object vector. After the next `scan()`:

- the vector is cleared
- interned paths are gone
- local maps are returned to the pool

Copy numbers and owned strings out if another scan will run. Printing and walking in the same breath as `scan()` is the intended use.

## Putting it together

The `sum` example stores each number on the child (`local`) and the total on `global`, then:

```rust
let root = res.root().unwrap();
let mut c = res.child(root).unwrap();
println!("{} => {:?}", res.path(c), res.local(c));
while let Some(next) = res.next_sibling(c) {
    c = next;
    println!("{} => {:?}", res.path(c), res.local(c));
}
```

That walks **only the direct children** of the text buffer (each `Number`). It does not need recursion because those children have no children. A folder of ZIPs of PNGs needs the recursive dump: folder → ZIP members → PNG nodes with `local` sizes.
