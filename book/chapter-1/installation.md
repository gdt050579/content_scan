# Installation

`content_scan` is a normal Cargo dependency. You only add the `content_scan` crate. The companion proc-macro crate (`content_scan_proc_macro`) is re-exported, so `#[derive(ContentType)]` and `#[derive(Dependencies)]` work after a single `use content_scan::*;`.

The crate targets the 2021 edition. You need a recent stable Rust toolchain with Cargo.

## From crates.io

In your package’s `Cargo.toml`:

```toml
[dependencies]
content_scan = "0.1"
```

That is the usual path once you are not developing against a local checkout.

## From a local checkout

This repository is a Cargo workspace. The library lives in the `content_scan/` member, not at the workspace root:

```toml
[dependencies]
content_scan = { path = "path/to/content_scan/content_scan" }
```

Point `path` at that inner crate directory (the one that contains its own `Cargo.toml`).

## From git

```toml
[dependencies]
content_scan = { git = "https://github.com/gdt050579/content_scan" }
```

Cargo resolves the `content_scan` package inside the workspace. Pin a `tag` or `rev` if you need a fixed snapshot.

## Prelude

Bring the public surface into scope:

```rust
use content_scan::*;
```

That import includes the traits (`Content`, `ContentIdentifier`, `ContentAnalyzer`, `ContentExtractor`, …), the ready-made content types (`BufferContent`, `FileContent`, `FolderContent`), the built-in folder and ZIP plugins, `ScannerBuilder`, `FilterBuilder`, the derive macros, and `var!` / `VarMap` from the re-exported [`varmap`](https://crates.io/crates/varmap) crate.

You do not add `content_scan_proc_macro` or `varmap` to your `Cargo.toml` unless you want them as direct dependencies for some other reason.

## Workspace layout (this repository)

If you are reading the source alongside the book, the repo has three members:

| Crate                     | Path                       | Role                                                                  |
| ------------------------- | -------------------------- | --------------------------------------------------------------------- |
| `content_scan`            | `content_scan/`            | Library: scanner, traits, matchers, filters, built-ins.               |
| `content_scan_proc_macro` | `content-scan-proc-macro/` | `#[derive(ContentType)]` and `#[derive(Dependencies)]`.               |
| `examples`                | `examples/`                | Runnable programs used again in [Examples](../chapter-6/examples.md). |

From the workspace root you can check that everything compiles:

```bash
cargo build
cargo test
cargo run --example vowals
```

The `vowals` example is the same shape as [Your first scan](first_scan.md): one content type, one identifier, one analyzer, one in-memory buffer.
