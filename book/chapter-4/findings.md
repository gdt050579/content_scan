# Findings and metadata

A **finding** is a detection attached to the object that was current when an analyzer recorded it: a hash, a YARA-like match, an entropy label, a packed-file warning. It is the flat “something was found here” channel, not a substitute for [`VarMap`s](global_vs_local.md).

```rust
context.add_finding("deadbeef", Some("ComputeHash"), None);
context.add_finding("packed", Some("entropy"), Some(Entropy(7.91)));
```

| Argument | Meaning |
| --- | --- |
| `finding` | The text (digest, message, class label, …). Interned into the scanner’s path arena. |
| `source` | Optional producer label (rule name, plugin id). `None` if unused. |
| `metadata` | Optional typed extra of type `M`. Pass `None` with the default [`NoMetadata`](#metadata). |

Calling `add_finding` when no object is current (outside `analyze`) is a no-op. Each stored finding keeps the object index, so later `Finding::path()` / `Finding::content_type()` resolve against the [result tree](scan_result.md).

## Findings versus maps

| | Finding | Local / global map |
| --- | --- | --- |
| Shape | Flat list, emission order | Keyed values on the scan or on one node |
| Typical payload | Short interned string + optional `M` | `Copy` struct, number, flag |
| How you read it | `for f in res.findings()` | `res.global().get` / `res.local(h).get` |
| Good for | Reports, detections, hashes | Aggregates, headers, dimensions |
| Bad for | The only copy of `{width, height}` you will query on a node | A thousand unrelated hit strings under one key |

An analyzer often does **both**: store `PeHeaders` locally for the next plugin, and `add_finding` when a signature hits.

The md5 example records each digest as a finding and iterates the list after `scan()`. The entropy example uses the finding **text** as a coarse class (`normal` / `encrypted` / `packed`) and the exact bits-per-byte as [metadata](#metadata). Image size does **not** use findings: width and height are local, printed while walking handles.

## How to process them

After `scan()`, iterate **once**, in the order analyzers called `add_finding`:

```rust
for f in res.findings() {
    println!(
        "{}  {}  [{}]  {:?}",
        f.finding(),
        f.path().unwrap_or("?"),
        f.source().unwrap_or("-"),
        f.content_type(),
    );
    if let Some(meta) = f.metadata() {
        // typed extras — see below
        let _ = meta;
    }
}
```

```rust
impl<'a, T: ContentType, M: FindingMetadata> Finding<'a, T, M> {
    pub fn finding(&self) -> &'a str;
    pub fn source(&self) -> Option<&'a str>;
    pub fn metadata(&self) -> Option<&'a M>;
    pub fn path(&self) -> Option<&'a str>;
    pub fn content_type(&self) -> Option<T>;
}
```

`path` is the same interned printable view as `ScanResult::path` for that object. There is no `ScanContentHandle` on a finding; if you need that object’s local map, walk the tree (or match on path), or put the structured data in metadata / local at record time.

The iterator and each `Finding` **borrow the `ScanResult`**. Do not start another `scan()` on the same scanner while they are in use. Copy strings you must keep.

There is no random access, no filter API on the list, and no grouping by path in the crate. Group in your own loop if you need “all hits on this file.” Emission order follows the scan: parent analyzers before their children, and within an object, analyzer priority order.

## Live processing: observer and `store_findings`

`add_finding` **always** notifies a [`ScanObserver::on_finding`](../chapter-3/observer.md) if one is attached.

[`store_findings`](../chapter-3/builder.md) (default `true`) only controls whether the hit is **kept** for `res.findings()`. Set it to `false` when the observer is the consumer — a log, a UI, a counter — and you do not want every digest interned until the scan ends. The `sha1` and `finder` examples do that.

Analyzers still call `add_finding`. They do not need to know whether the list will be retained. If you turn storage off, `res.findings()` is empty even though `on_finding` ran.

## Metadata

The type parameter `M` is the same on `Scanner`, `Context`, `ScanResult`, and `ContentAnalyzer`. You cannot mix metadata types on one scanner.

`ScannerBuilder::new()` uses [`NoMetadata`](../chapter-3/builder.md#new-vs-with_metadata). Analyzers then pass `None` as the third argument.

For typed extras (severity, offset, entropy, rule id), implement the marker trait — **`Copy` is required** so findings stay cheap to store and iterate — and start from `with_metadata`:

```rust
#[derive(Copy, Clone, Debug)]
struct Entropy(f64);
impl FindingMetadata for Entropy {}

impl ContentAnalyzer<MyTypes, Entropy> for EntropyAnalyzer {
    fn analyze(
        &mut self,
        content: &mut dyn Content<MyTypes>,
        context: &mut Context<MyTypes, Entropy>,
    ) -> NextAction {
        let h = shannon(content);
        let label = if h > 7.8 { "packed" } else { "normal" };
        context.add_finding(label, None, Some(Entropy(h)));
        NextAction::Continue
    }
}

let mut scanner = ScannerBuilder::<MyTypes>::with_metadata::<Entropy>()
    .add_generic_analyzer(0, EntropyAnalyzer {})
    .build();
```

Keep `M` small. A path string or a heap buffer is not metadata; interned `finding` / `source` already cover labels, and large payloads belong on a child `Content`.

## What this page leaves out

Walking from a finding to siblings and children is [Navigating ScanResult](scan_result.md). When a hit should become its own scanned object instead of a string on the parent, that is [Requesting extraction](../chapter-2/requesting_extraction.md).
