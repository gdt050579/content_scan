# Analyzer

An analyzer **inspects** an object that has already been through identification (or, for generic analyzers, any object) and **records** what it learned. It does not return “the scan result.” It writes into the [`Context`](../chapter-4/context.md) — maps, findings, extraction requests — and returns a [`NextAction`](#nextaction) that steers the rest of *this* object.

You may register **many** analyzers, including several for the same type. That is how a PE scan can parse headers in one plugin and extract icons in another, without either pretending to be a monolith.

```rust
pub trait ContentAnalyzer<T: ContentType, M: FindingMetadata = NoMetadata>: Dependencies {
    fn analyze(
        &mut self,
        content: &mut dyn Content<T>,
        context: &mut Context<T, M>,
    ) -> NextAction;
}
```

Every analyzer must implement [`Dependencies`](dependencies.md) (almost always with `#[derive(Dependencies)]`). Identifiers classify; extractors produce children. Analyzers are the place domain knowledge lands.

## Registration

```rust
.add_analyzer(MyTypes::Pe, 0, PeHeaderAnalyzer::new())
.add_analyzer(MyTypes::Pe, 10, PeIconAnalyzer {})
.add_generic_analyzer(20, SignatureAnalyzer::from_file("rules.bin"))
```

- **`add_analyzer(type, priority, analyzer)`** — runs only when the object was identified as `type`.
- **`add_generic_analyzer(priority, analyzer)`** — runs on **every** object, including unidentified ones and folders, **after** that object’s typed analyzers.

`priority` is `0..=255`. **Lower runs first.** Typed analyzers for the resolved type all run (by priority), then the generic bucket (by priority). Two analyzers with the same `(type, priority)` both run; their relative order is unspecified.

The value you pass to `add_analyzer` is **your instance**. Construction happens at builder time, once; the scanner keeps that instance for every later `scan()`. That is where you load signatures, rule files, or other tables — see [Loading data at builder time](#loading-data-at-builder-time).

## `NextAction`

Only analyzers return this. Extractors yield `Option`.

| Value | Effect |
| --- | --- |
| `Continue` | Next analyzer for this object; after the last one, extractors. |
| `Skip` | Stop **this** object: remaining analyzers and extractors on it do not run. Siblings and later objects still scan. |
| `Exit` | Abort the **entire** scan. The call stack unwinds back to `scan()`. |

`Skip` is “I am done with this file.” `Exit` is “stop everything.” Default to `Continue`.

Aborting from **outside** the analyzer (timeout, cancel button) is a [`StopCondition`](../chapter-3/stop_condition.md), not `NextAction`. That check runs before the object is identified; `Exit` from `analyze` runs after it is already in the tree.

## Writing the context for another analyzer

The `Context` is shared across all analyzers on the current object. A plugin that has already parsed something should **store it**, so a later plugin can use it instead of walking the file again.

That is the usual PE split: one analyzer reads the DOS stub, `e_lfanew`, COFF/optional header, and section table, and puts a small struct in the **local** map. An icon (or resource, or import) analyzer then reads that struct and follows the data directories — it does not re-parse `MZ` / `PE\0\0`.

Use the **local** map for per-object data (`context.local()`). Use the **global** map for scan-wide aggregates (`context.global()`). The APIs (`var!`, `VarMapValue`, looking values up after the scan) are [Chapter 4](../chapter-4/global_vs_local.md). Here the important rule is: **whatever you `set` is visible to analyzers that run later on the same object.**

```rust
#[derive(Debug, Clone, VarMapValue)]
struct PeHeaders {
    e_lfanew: u32,
    number_of_sections: u16,
    resource_rva: u32,
    // ...
}

#[derive(Dependencies)]
#[Dependencies(name = "PeHeaders")]
struct PeHeaderAnalyzer;

impl ContentAnalyzer<MyTypes> for PeHeaderAnalyzer {
    fn analyze(
        &mut self,
        content: &mut dyn Content<MyTypes>,
        context: &mut Context<MyTypes>,
    ) -> NextAction {
        let Some(headers) = parse_pe_headers(content) else {
            return NextAction::Continue;
        };
        context.local().set(var!("pe_headers"), headers);
        NextAction::Continue
    }
}

#[derive(Dependencies)]
#[Dependencies(name = "PeIcons", requires = "PeHeaders")]
struct PeIconAnalyzer;

impl ContentAnalyzer<MyTypes> for PeIconAnalyzer {
    fn analyze(
        &mut self,
        content: &mut dyn Content<MyTypes>,
        context: &mut Context<MyTypes>,
    ) -> NextAction {
        let Some(headers) = context.local().get::<PeHeaders>(var!("pe_headers")).cloned() else {
            return NextAction::Continue;
        };
        // Follow headers.resource_rva — do not parse the PE again.
        if let Some(count) = extract_icon_count(content, &headers) {
            context.local().set(var!("icon_count"), count);
        }
        NextAction::Continue
    }
}
```

Register the producer with a **strictly smaller** priority than the consumer:

```rust
ScannerBuilder::new()
    .add_identifier(MyTypes::Pe, PeIdentifier {})
    .add_analyzer(MyTypes::Pe, 0, PeHeaderAnalyzer {})
    .add_analyzer(MyTypes::Pe, 10, PeIconAnalyzer {})
    .build();
```

`requires = "PeHeaders"` names the other analyzer’s `Dependencies` `name`. In debug builds, `build()` checks that a plugin with that name is registered and that its priority is strictly smaller. Details are on [Dependencies](dependencies.md).

Without that order, `PeIconAnalyzer` would see an empty local map and skip or re-parse. The context is the blackboard; priority is who writes first.

## Findings

**Any analyzer can emit findings** — typed or generic, early or late, whether or not it also writes maps. Findings are the flat “something was detected here” list: a hash, a YARA-like match, an entropy label, a packed-file warning. Each finding is attached to the object that was current when it was recorded.

```rust
context.add_finding("packed", Some("entropy"), None);
context.add_finding(hash.as_str(), None, None);
```

The three arguments are the text, an optional source label (plugin or rule name), and optional typed [metadata](../chapter-4/findings.md). After `scan()`, iterate `res.findings()` — unless you built the scanner with [`store_findings(false)`](../chapter-3/observer.md#findings-without-storing-them), in which case the list is empty and an [observer](../chapter-3/observer.md) is how you see the hits. Maps are for structured fields you will query on a node (`pe_headers`, `icon_count`). Findings are for the list you print or ship as detections. An analyzer often does both: store headers locally, and `add_finding` when an icon or a signature hits.

The md5 example in the repo is a generic analyzer whose only output is findings, iterated after the scan. The `sha1` example is the same hash pattern with an observer and `store_findings(false)`. The PE icon analyzer above could `add_finding` for each extracted icon as well as storing `icon_count`.

Treat the storage shape as a general notion here. [Chapter 4](../chapter-4/context.md) is the full API: [maps](../chapter-4/global_vs_local.md), [findings and metadata](../chapter-4/findings.md), and [walking the result tree](../chapter-4/scan_result.md).

## Loading data at builder time

`analyze` runs once per object, possibly millions of times. Tables that do not depend on the current file — signature databases, rule sets, unpacker configs — belong on the **analyzer struct**, loaded when you **construct** the plugin, then moved into `ScannerBuilder`.

The `Context` is **cleared at the start of every `scan()`**. Putting signatures in `context.global()` would reload or vanish between inputs. The scanner instance is reused; the analyzer instance is too.

```rust
struct SignatureAnalyzer {
    rules: Vec<Rule>,
}

impl SignatureAnalyzer {
    fn from_file(path: &str) -> Self {
        Self {
            rules: load_rules(path), // I/O once, at setup
        }
    }
}

impl ContentAnalyzer<MyTypes> for SignatureAnalyzer {
    fn analyze(
        &mut self,
        content: &mut dyn Content<MyTypes>,
        context: &mut Context<MyTypes>,
    ) -> NextAction {
        for rule in &self.rules {
            if rule.matches(content) {
                context.add_finding(rule.name, Some("signatures"), None);
            }
        }
        NextAction::Continue
    }
}

fn main() {
    let mut scanner = ScannerBuilder::new()
        .add_generic_analyzer(20, SignatureAnalyzer::from_file("rules.bin"))
        .build();
    // scanner.scan(...) as many times as you want — rules stay loaded
}
```

`from_file` (or `new`, `from_bytes`, …) runs in `main` **before** `build()`. `add_generic_analyzer` / `add_analyzer` take ownership of that value. After `build()`, you do not get it back; load everything you need in the constructor.

That pattern pairs naturally with the PE blackboard: header parsing is per-object (local map), signature bytes are process-lifetime (fields on the analyzer).

## Typed vs generic, briefly

A PE header analyzer is **typed**: it only makes sense on `MyTypes::Pe`. A signature or hash analyzer is often **generic**: it should see every file, regardless of type (skip folders inside `analyze` if they have no bytes). Generic analyzers still write the same context and the same findings list.

## Requesting extraction

An analyzer that finds nested content of **another** type — an embedded ZIP inside a PE — does not unpack it itself. It calls `context.request_extract(MyTypes::Zip).at(offset).len(len).emit()`. After this object’s analyzers finish, extractors registered for `Zip` run on that window. That mechanism is [Requesting extraction](requesting_extraction.md). Small structs for the next analyzer stay in the scan context instead — [Extractions vs Context](extractions_vs_context.md).

## What this page leaves out

- Full `var!` / `VarMap` / result-tree APIs — [Chapter 4](../chapter-4/context.md).
- `Skip` / `Exit` versus nested extraction sessions — [How one scan runs](../chapter-3/how_one_scan_runs.md).
- The `#[Dependencies]` attribute, debug checks, and the global name space — [Dependencies](dependencies.md).
