# Dependencies

Every [`ContentAnalyzer`](analyzer.md) implements `Dependencies`. The usual form is a derive plus a helper attribute:

```rust
#[derive(Dependencies)]
#[Dependencies(name = "PeHeaders")]
struct PeHeaderAnalyzer;

#[derive(Dependencies)]
#[Dependencies(name = "PeIcons", requires = "PeHeaders")]
struct PeIconAnalyzer;
```

- **`name`** is required and must be non-empty. Other analyzers refer to this plugin by that string in `requires`.
- **`requires`** is optional. A single string or an array of strings: analyzers that must run **first**.

```rust
#[Dependencies(name = "NeedsHash", requires = ["OpenFile", "ComputeHash"])]
```

These names are **not** `ContentType` variants. They are plugin ids. Typed and generic analyzers share one name space.

## Why it exists

Analyzers share a [`Context`](../chapter-4/context.md). A later plugin can read what an earlier one wrote — PE headers for an icon extractor, a hash for a “known malware” checker. That only works if the producer actually ran first.

`requires` documents that order. `priority` on `add_analyzer` / `add_generic_analyzer` is what the scanner actually uses (lower first). The derive lets debug builds **check** that the registration matches the declaration.

## What `build()` checks (debug only)

`name()` and `dependencies()` exist only when `debug_assertions` are enabled. In debug builds, `ScannerBuilder::build` verifies that:

1. Every name listed in `requires` is a registered analyzer (typed or generic).
2. Each required analyzer has a **strictly smaller** `priority` than the one that requires it.

Release builds skip the check. Duplicate `name`s among registered analyzers are not rejected; the last registration wins for the debug map.

If `PeIcons` requires `PeHeaders` and you register both at priority `0`, debug `build()` panics. Give the producer a smaller priority:

```rust
.add_analyzer(MyTypes::Pe, 0, PeHeaderAnalyzer {})
.add_analyzer(MyTypes::Pe, 10, PeIconAnalyzer {})
```

The check is by **priority value**, not by “typed before generic.” A generic analyzer at priority `5` can require a typed analyzer at priority `0`, and the other way around, as long as the numbers work. Remember that **on a given object** typed analyzers still run as a group before generics; `requires` will not make a generic plugin run in the middle of the typed list. If the consumer must see the producer’s local map, put both in the same bucket (both typed for `Pe`, or both generic) and order them with priority.

## Derive limits

Generics are not supported. Unit structs, tuple structs, named-field structs, enums, and unions all work. `name` must be a string literal.

The [Analyzer](analyzer.md) page shows the PE header → icon pattern this trait is for.
