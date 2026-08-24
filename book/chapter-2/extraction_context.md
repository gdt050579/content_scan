# Extraction Context

`ExtractionContext` is what [`create_session`](extractor.md) receives. It names a **region of the parent** plus an optional parameter map. It is valid **only for that call** — copy `offset`, `length`, and any `params` you need onto the session.

```rust
pub struct ExtractionContext<'a> {
    pub offset: u64,                // first byte within the parent
    pub length: Option<u64>,        // Some(n) = known size; None = you decide
    pub params: Option<&'a VarMap>, // analyzer extras, or None
}
```

There is no second constructor. The scanner fills the struct in two different situations; the fields mean the same thing in both.

## Type-specific run

The parent was **identified as** this extractor’s type. The context covers the **whole object**:

| Field | Value |
| --- | --- |
| `offset` | `0` |
| `length` | `Some(content.size())` |
| `params` | `None` |

A `FolderExtractor` or `ZipExtractor` on a file that *is* a folder or a ZIP sees this. You can ignore the context if you always walk the entire parent.

## Requested run

An analyzer queued this type with [`request_extract`](requesting_extraction.md). The parent does **not** have to be that type. The fields come from the request:

| Field | Value |
| --- | --- |
| `offset` | `.at(...)` (default `0` if omitted) |
| `length` | `.len(...)`, or `None` if the analyzer omitted it |
| `params` | map from `.param(...)`, or `None` if none were set |

`length = None` means the extractor determines the extent itself (parse until the format ends, scan to EOF). `Some(n)` is an assertion that the region is `n` bytes.

## `params`

A password, codec, overlay flag, or other small extra belongs here — not in the scan-wide [`Context`](../chapter-4/context.md). The borrow lasts only for `create_session`. Copy values into the session if `advance` / `extract` need them later.

```rust
fn create_session(
    &mut self,
    content: OwnedContentPtr<MyTypes>,
    ctx: &ExtractionContext,
) -> Option<Box<dyn ExtractionSession<MyTypes>>> {
    let start = ctx.offset;
    let len = ctx.length; // None = parse until the format ends
    let password = ctx
        .params
        .and_then(|p| p.get::<&str>(var!("password")))
        .map(str::to_string);
    Some(Box::new(ZipSession {
        content,
        start,
        len,
        password,
        entry: Entry::default(),
        // ...
    }))
}
```

How an analyzer fills this struct is [Requesting extraction](requesting_extraction.md). When to use an extractor at all, versus stuffing a small struct into the scan `Context`, is [Extractions vs Context](extractions_vs_context.md).
