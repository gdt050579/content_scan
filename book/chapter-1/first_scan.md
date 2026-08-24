# Your first scan

This page builds a complete program: define a content type, identify a buffer by magic bytes, count vowels, and read the result. It is the same shape as the `vowals` example in the repository. Later chapters explain each API in full; here the point is to see a scan run end to end.

Create a binary crate, add `content_scan` as shown in [Installation](installation.md), and put the following in `src/main.rs`.

## 1. Define the kinds of content you know

Every scanner is parameterized by a user enum that implements `ContentType`. The derive macro fills in the trait if the enum is `#[repr(u16)]` with unit variants and you also derive `Copy`, `Eq`, `Ord`, and the usual companions:

```rust
use content_scan::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ContentType)]
#[repr(u16)]
enum MyType {
    TextBuffer,
}
```

This scanner only knows one kind. Real tools add variants for folders, images, archives, and so on. See [ContentType](../chapter-2/content_type.md).

## 2. Identify the buffer

An identifier answers “is this object that type?” The fast path is an `IdentifyMethod`: magic bytes, file name, or extension. After a fast match, `validate` can accept or reject the candidate. Returning `true` is enough when the magic is already decisive:

```rust
struct TextBufferIdentifier;

impl ContentIdentifier<MyType> for TextBufferIdentifier {
    fn identify_method(&self) -> Option<IdentifyMethod> {
        Some(IdentifyMethod::Magic(b"TXBF"))
    }

    fn validate(&self, _: &mut dyn Content<MyType>) -> bool {
        true
    }
}
```

`Magic` patterns are exact and at most 16 bytes — that is the window the scanner reads. Longer signatures belong in `validate` via `content.read(...)`. Details are in [Identifier](../chapter-2/identifier.md).

## 3. Analyze it

An analyzer reads the object and writes into a `Context`. Every analyzer must implement `Dependencies` (almost always with the derive). The `name` is what other analyzers refer to in `requires`; this first example has no dependencies.

```rust
#[derive(Dependencies)]
#[Dependencies(name = "VowelAnalyzer")]
struct VowelAnalyzer;

impl ContentAnalyzer<MyType> for VowelAnalyzer {
    fn analyze(
        &mut self,
        content: &mut dyn Content<MyType>,
        context: &mut Context<MyType>,
    ) -> NextAction {
        let mut count = 0u32;
        // Skip the four-byte magic prefix.
        for i in 4..content.size() {
            if let Some(b) = content.read(i, 1) {
                let b = b[0].to_ascii_lowercase();
                if matches!(b, b'a' | b'e' | b'i' | b'o' | b'u') {
                    count += 1;
                }
            }
        }
        context.global().set(var!("count_vowels"), count);
        NextAction::Continue
    }
}
```

A few things to notice:

- `content.read(offset, count)` returns a borrowed slice, or `None` if that window cannot be read.
- `context.global()` is a scan-wide [`VarMap`](../chapter-4/global_vs_local.md). `var!("count_vowels")` is a compile-time typed key.
- `NextAction::Continue` means “keep going on this object” (remaining analyzers, then extractors). `Skip` stops this object; `Exit` aborts the whole scan.

## 4. Build the scanner and run it

Register the identifier and the analyzer, wrap the bytes in `BufferContent`, and call `scan`. The `0` next to the analyzer is its **priority**: lower numbers run first. The `true` passed to `scan` means “apply the filter to the root.” There is no filter here, so it has no effect.

```rust
fn main() {
    let mut scanner = ScannerBuilder::new()
        .add_identifier(MyType::TextBuffer, TextBufferIdentifier {})
        .add_analyzer(MyType::TextBuffer, 0, VowelAnalyzer {})
        .build();

    let mut content = BufferContent::<MyType>::new(b"TXBF hello", "test.txt");
    let result = scanner.scan(&mut content, true);

    let count = result.global().get::<u32>(var!("count_vowels")).unwrap();
    println!("scanned {} object(s), vowels = {}", result.objects_scanned(), count);
}
```

`BufferContent::new` copies the slice and leaves the content type unset, so the scanner must identify the object. The path `"test.txt"` is a synthetic UTF-8 address, not a file on disk.

Run it:

```bash
cargo run
```

You should see:

```text
scanned 1 object(s), vowels = 2
```

(`hello` contributes `e` and `o`. The `TXBF` prefix is not counted.)

## What just happened

For that single object the scanner:

1. Built an empty result context (a scanner is reusable; each `scan` starts clean).
2. Did not filter the root — no `Filter` was configured.
3. Saw that `content_type()` was `None`, read the first bytes, matched `TXBF`, and accepted `MyType::TextBuffer` after `validate` returned `true`.
4. Ran `VowelAnalyzer` (the only analyzer registered for that type).
5. Found no extractors and no extra extraction requests, so it stopped.
6. Returned a `ScanResult` that still borrows the scanner’s context. Read what you need before the next `scan` on the same instance.

If you had constructed the buffer with `BufferContent::with_content_type(..., MyType::TextBuffer)`, step 3 would have been skipped: a known type is used as-is and identifiers do not run.

## The full listing

```rust
use content_scan::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ContentType)]
#[repr(u16)]
enum MyType {
    TextBuffer,
}

struct TextBufferIdentifier;

impl ContentIdentifier<MyType> for TextBufferIdentifier {
    fn identify_method(&self) -> Option<IdentifyMethod> {
        Some(IdentifyMethod::Magic(b"TXBF"))
    }

    fn validate(&self, _: &mut dyn Content<MyType>) -> bool {
        true
    }
}

#[derive(Dependencies)]
#[Dependencies(name = "VowelAnalyzer")]
struct VowelAnalyzer;

impl ContentAnalyzer<MyType> for VowelAnalyzer {
    fn analyze(
        &mut self,
        content: &mut dyn Content<MyType>,
        context: &mut Context<MyType>,
    ) -> NextAction {
        let mut count = 0u32;
        for i in 4..content.size() {
            if let Some(b) = content.read(i, 1) {
                let b = b[0].to_ascii_lowercase();
                if matches!(b, b'a' | b'e' | b'i' | b'o' | b'u') {
                    count += 1;
                }
            }
        }
        context.global().set(var!("count_vowels"), count);
        NextAction::Continue
    }
}

fn main() {
    let mut scanner = ScannerBuilder::new()
        .add_identifier(MyType::TextBuffer, TextBufferIdentifier {})
        .add_analyzer(MyType::TextBuffer, 0, VowelAnalyzer {})
        .build();

    let mut content = BufferContent::<MyType>::new(b"TXBF hello", "test.txt");
    let result = scanner.scan(&mut content, true);

    let count = result.global().get::<u32>(var!("count_vowels")).unwrap();
    println!("scanned {} object(s), vowels = {}", result.objects_scanned(), count);
}
```

## Where to go next

This example never extracted children, never walked a directory, and never emitted a [`Finding`](../chapter-4/findings.md). Those show up as soon as you need containers or a flat list of hits.

- [Basic concepts](../chapter-2/basic_concepts.md) — `Content`, identifiers, analyzers, and extractors in detail.
- [How one scan runs](../chapter-3/how_one_scan_runs.md) — the exact order of filter → identify → analyze → extract → recurse.
- [Examples](../chapter-6/examples.md) — the same ideas on files, folders, ZIP members, and requested extraction.
