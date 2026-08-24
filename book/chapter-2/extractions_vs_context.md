# Extractions vs Context

Analyzers have two ways to hand work to the rest of the scan:

- Write a value into the scan [`Context`](../chapter-4/context.md) (`local` / `global` maps, findings) so **another analyzer on the same object** can read it.
- Queue an [extraction](extractor.md) so a **new child `Content`** is created and run through the pipeline.

They are not interchangeable. The rule of thumb is **size and whether it should be an object**.

## Use the Context

Use `context.local()` (and `requires` / priority) for **small, copyable structures** that later analyzers on **this** object need:

- PE / COFF headers, section tables, data-directory RVAs
- A decrypted or reconstructed **path** or short name
- Dimensions, checksums, flags, a small decoded blob that is still “metadata”
- Anything you would happily `Clone` into a `VarMap`

Those values never become their own node in the result tree. They are not filtered as files. They are not identified. The next analyzer just `get`s them and continues reading the same parent.

The [PE header → icon](analyzer.md#writing-the-context-for-another-analyzer) pattern is this: parse once, store `PeHeaders` locally, the icon analyzer follows `resource_rva` without touching `MZ` again.

`context.global()` is the same idea at scan scope (totals, flags), not a substitute for children. Findings are detections on the current object, not payloads.

## Use an extractor

Use an extractor (type-specific or [requested](requesting_extraction.md)) when the thing is **content in its own right**:

- An **entire file** (ZIP member, directory entry, overlay, decoded Base64 payload)
- Something you may **filter** by path or size (`*.png`, skip huge blobs)
- Something you may **drop** as out of scope (`advance` advertised it; `extract` returned `None`, or the filter rejected the `Entry`)
- A **view** over a region of the parent — including one that caches a window in memory — that later plugins should see as a normal `Content` (identify, analyze, extract again)

Children get a [`ContentPath`](content_path.md), a size, a depth, and the full pipeline. That cost is the point: you want the rest of the scanner (filters, identifiers, typed analyzers, nested extractors, the result tree) to treat them as objects.

A custom `Content` that wraps the parent at `(offset, length)` and caches the first N bytes is still an extraction. It is a child, not a `VarMap` entry.

## Side by side

| | Context (maps) | Extraction |
| --- | --- | --- |
| What you store | A small struct / string / number | A `Content` child |
| Who reads it | Later **analyzers on the same object** | The **pipeline** (filter, identify, analyze, extract) |
| Filter / `max_depth` | No | Yes |
| Result tree | Stays on the parent’s local (or global) map | New node (parent / child / sibling) |
| Typical size | Headers, paths, counts | Files, members, decoded payloads, memory views |
| Typical API | `context.local().set(var!("pe_headers"), …)` | `create_session` / `request_extract(…).emit()` |

A decrypted **path string** used by the next PE analyzer → Context. A decrypted **file body** that might be a ZIP or a PNG → extract it, then let identification and the filter decide.

If you are unsure, ask: *should this appear as its own row in the scan tree, and could I want to skip it by extension or size?* If yes, extract. If it is only scaffolding for the next function on the same bytes, put it in the Context.
