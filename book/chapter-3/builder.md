# Builder

`ScannerBuilder<T>` (optionally `ScannerBuilder<T, M>`) is how a [`Scanner`](scanner.md) comes into existence. Register plugins, optionally attach a filter and a depth limit, then `build()`.

```rust
let mut scanner = ScannerBuilder::<MyTypes>::new()
    .filter(filter)                          // optional
    .max_depth(8)                            // default: 8
    .add_identifier(MyTypes::Pe, PeIdentifier {})
    .add_analyzer(MyTypes::Pe, 0, PeHeaderAnalyzer {})
    .add_generic_analyzer(20, HashAnalyzer {})
    .add_extractor(MyTypes::Zip, ZipExtractor::new())
    .build();
```

After `build()` the builder is gone. There is no `add_analyzer` on `Scanner`. Changing the plugin set means building a new scanner. That matches [create once, scan many times](scanner.md#create-once-scan-many-times): assembly is the expensive, one-shot step.

## `new` vs `with_metadata`

| Start                                             | Finding extras                                                                                          |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `ScannerBuilder::<MyTypes>::new()`                | [`NoMetadata`](../chapter-4/findings.md) — analyzers pass `None` as the third argument to `add_finding` |
| `ScannerBuilder::<MyTypes>::with_metadata::<M>()` | Custom `M: FindingMetadata` (must be `Copy`)                                                            |

`new()` is defined only for `NoMetadata` so `M` can be inferred. The entropy example uses `with_metadata::<Entropy>()`. You cannot mix metadata types on one scanner; `M` is a type parameter of `Scanner` itself.

## Registration

| Method                              | Meaning                                                                          |
| ----------------------------------- | -------------------------------------------------------------------------------- |
| `add_identifier(ty, id)`            | At most **one** per `ty`. Fast `IdentifyMethod` goes into compiled matchers.     |
| `add_analyzer(ty, priority, a)`     | Typed analyzer. `priority` `0..=255`, lower first. Several per type are allowed. |
| `add_generic_analyzer(priority, a)` | Runs on every object, after typed analyzers.                                     |
| `add_extractor(ty, e)`              | Typed only. Several per type, **registration order** (no priority).              |
| `filter(f)`                         | Replaces any previous filter.                                                    |
| `max_depth(n)`                      | Clamped to `1..=u32::MAX - 2`. Default `8`.                                      |

The value you pass in is **moved** into the builder: signature tables and open configs live on those instances for the lifetime of the scanner. See [Loading data at builder time](../chapter-2/analyzer.md#loading-data-at-builder-time).

Plugin cardinality and order are in [Architecture](../chapter-2/architecture.md). Analyzer `requires` / debug checks are in [Dependencies](../chapter-2/dependencies.md).

## What `build()` does

1. Panics if two identifiers share a `ContentType`.
2. Panics if a `Magic` / `MultipleMagic` pattern is longer than 16 bytes.
3. In **debug** builds, panics if an analyzer `requires` a name that is not registered, or if that dependency does not have a strictly smaller `priority`.
4. Compiles identifier matchers (one / packed magic / trie).
5. Sorts analyzers into per-type and generic ranges; extractors into per-type ranges.

Release builds skip the dependency check. Duplicate analyzer `name`s are not rejected.

There is no fallible `try_build`. Illegal registrations are programming errors and panic on purpose.

## Defaults

An empty builder is legal: no identifiers, no analyzers, no extractors, no filter, `max_depth` 8. `scan()` then records objects (and runs nothing on them) unless the root is filtered out. Useful for tests; production scanners register at least the plugins they need.
