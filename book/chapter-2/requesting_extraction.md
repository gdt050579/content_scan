# Requesting extraction

An analyzer that finds nested content of **another** type does not unpack it itself. It queues a pass: after this object’s analyzers finish, the scanner runs extractors registered for that type on the **same** parent, with an [`ExtractionContext`](extraction_context.md) describing the window.

The parent does **not** need to have been identified as the requested type. A PE stays a `Pe`; ZIP extractors still run on the overlay at `0x1000`. A text file stays `Text`; a Base64 extractor still decodes a slice in the middle.

That is the second of the two times an [extractor](extractor.md) runs. The first is “this object *is* a ZIP / folder / …” and uses a whole-object context. This page is the analyzer-driven case.

## The builder

`context.request_extract(ty)` returns an `ExtractRequestBuilder`. Chain setters and commit with `.emit()`. Dropping the builder without `emit` does nothing (the type is `#[must_use]`).

```rust
impl ContentAnalyzer<MyTypes> for PeAnalyzer {
    fn analyze(
        &mut self,
        _: &mut dyn Content<MyTypes>,
        context: &mut Context<MyTypes>,
    ) -> NextAction {
        context
            .request_extract(MyTypes::Zip)
            .at(0x1000)
            .len(4096)
            .param(var!("password"), "secret")
            .emit();
        NextAction::Continue
    }
}
```

| Method | Effect |
| --- | --- |
| `at(offset)` | Byte offset within the parent. Defaults to `0`. |
| `len(n)` | Asserts the region is `n` bytes. Omit to leave `length = None` (the extractor decides). |
| `param(key, value)` | One extractor-specific extra. Repeatable. The first call reserves a pooled `VarMap`; later calls write into the same map. No `.param()` → `params = None`. |
| `emit()` | Commits the request. Required. |

`value` must implement `VarMapValue` (the same bound as context maps). Keys are usually `var!("...")`.

## When it runs

After **all** analyzers for the current object have returned `Continue`:

1. Extractors registered for the object’s **own** identified type (whole object).
2. Then, in **emission order**, extractors registered for each **requested** type, with that request’s `ExtractionContext`.

`Skip` or `Exit` from an analyzer skips extractors on this object, including queued requests. Several requests (same or different types) may be emitted from one analyzer; each is independent. The queue is **cleared at the start of every object**, including nested children — a child does not inherit the parent’s requests.

If nothing is registered for the requested type, the request is ignored (any param map is released). Register the extractor:

```rust
ScannerBuilder::new()
    .add_identifier(MyTypes::Pe, PeIdentifier {})
    .add_analyzer(MyTypes::Pe, 0, PeAnalyzer {})
    .add_extractor(MyTypes::Zip, ZipExtractor::new())
    .build();
```

The ZIP extractor is registered for `Zip`, not for `Pe`. It runs on the PE because the analyzer asked, not because the file was identified as a ZIP.

## Copying the window in `create_session`

The extractor sees the request only as an `ExtractionContext`. Copy it off immediately:

```rust
fn create_session(
    &mut self,
    content: OwnedContentPtr<MyTypes>,
    ctx: &ExtractionContext,
) -> Option<Box<dyn ExtractionSession<MyTypes>>> {
    let start = ctx.offset;
    let len = ctx.length.unwrap_or(content.size().saturating_sub(start));
    if len == 0 {
        return None;
    }
    Some(Box::new(Base64Session {
        content,
        offset: start,
        length: len,
        done: false,
        entry: Entry::default(),
    }))
}
```

That is the shape of the [base64 example](../chapter-6/base64.md): a `Text` analyzer locates runs and `request_extract(Base64).at(start).len(len).emit()`; the `Base64` extractor decodes that slice into a child pinned as `Base64Decoded`. The parent file is never identified as `Base64`.

## What a request is not

It is not a way to pass PE headers to the next analyzer. Those stay in `context.local()` — see [Extractions vs Context](extractions_vs_context.md). A request creates **child `Content` objects**: they get a path, a filter check, a place in the result tree, and their own identify / analyze / extract pass (subject to `max_depth`).
