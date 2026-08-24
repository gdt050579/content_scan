# Identifier

An identifier answers one question: **what `ContentType` is this object?** The scanner asks before it runs typed analyzers or extractors. If `Content::content_type()` already returns `Some(ty)`, identifiers are skipped entirely — see [Content](content.md).

There is **at most one identifier per variant**. A second `add_identifier` for the same type panics in `build()`. Identification must produce a single type (or none), not a list.

```rust
pub trait ContentIdentifier<T: ContentType> {
    fn identify_method(&self) -> Option<IdentifyMethod>;
    fn validate(&self, content: &mut dyn Content<T>) -> bool;
}
```

The two methods are a **fast pre-filter** plus a **confirmation**. That split is the whole design.

## Why two steps

A PNG, a ZIP, and a PE can often be guessed from a handful of bytes or from a file name. Doing that guess with a full parser on every object in a directory tree is expensive. Doing *only* the guess is too loose: many files start with `MZ` that are not Portable Executables, and a `.exe` name is not proof either.

So the scanner:

1. Runs a **fast check** compiled from `identify_method()` — magic bytes, file name, or extension. No plugin code runs here. The matchers are built once in `ScannerBuilder::build` and reused for every object.
2. Calls **`validate`** on the identifier that owns the candidate type. This is your chance to read more of the object and reject a false positive. Returning `false` discards that candidate; the scanner tries the next fast method.

`validate` is not optional in the trait, but it can be the trivial `true` when the fast check is already decisive (the four-byte `TXBF` tag in [Your first scan](../chapter-1/first_scan.md) is that case). For real file formats it is where the identifier becomes trustworthy.

## Fast checks: `IdentifyMethod`

```rust
pub enum IdentifyMethod {
    Magic(&'static [u8]),
    MultipleMagic(&'static [&'static [u8]]),
    Extension(&'static str),
    Extensions(&'static [&'static str]),
    Name(&'static str),
    Names(&'static [&'static str]),
}
```

Return `Some(...)` from `identify_method` to register one of these. Return `None` to skip the fast path and live in `validate` only (those identifiers run **last**).

### Magic

`Magic` / `MultipleMagic` match a **prefix** of the content: the scanner reads `content.read(0, 16)` and asks whether those bytes start with a registered pattern.

- Patterns are **exact** byte sequences, not case-folded.
- Each pattern is at most **16 bytes**. That is the window the scanner reads. `build()` panics if a registered magic is longer. Bytes past offset 16 belong in `validate`.
- Overlapping prefixes resolve to the **longest** match.

> **Observation.** That `read(0, 16)` happens **only if at least one identifier registered a `Magic` or `MultipleMagic` pattern**. A scanner that uses only file names, extensions, or `validate`-only identifiers never opens the payload just to classify it. Name and extension checks look at the path alone.

`IdentifyMethod::Magic(b"MZ")` is the classic PE / DOS fast check: two bytes, cheap, and shared by a lot of things that are not a PE. That is exactly why `validate` exists — see below.

`ZipIdentifier` uses the local-file magic `PK\x03\x04` the same way, then confirms an End of Central Directory in `validate`.

### File name

`Name` / `Names` match the **basename** (the last segment of [`ContentPath::as_bytes`](content_path.md)), ASCII case-insensitive, exact string. `makefile` and `MAKEFILE` both match `Name("Makefile")`. This is for well-known names (`Makefile`, `Dockerfile`, `Cargo.toml` as a name — not as an extension), not for “files that look like text.”

### Extension

`Extension` / `Extensions` match the bytes after the last `.` in the basename, **without** the dot, ASCII case-insensitive. `Notes.TXT` matches `Extension("txt")`; a registered `"JPG"` matches `photo.jpg`.

Extension is the weakest of the three fast checks. A renamed PE still starts with `MZ`; a `.exe` that is a script does not. Prefer magic when the format has one, and still `validate`.

## Order of candidates

For an object whose type is not already pinned, identification is **two phases**. First the scanner **computes** the three fast matches (at most one type each). Then it **`validate`s hits** in a fixed order. Custom identifiers (`identify_method` → `None`) run only if that second phase produces no type.

**Phase 1 — compute matches** (all of them, up front):

1. **File name** against the basename matcher.
2. **Extension** against the extension matcher.
3. **Magic**, but only if at least one identifier registered `Magic` / `MultipleMagic`. Then `read(0, 16)` and the magic matcher. Otherwise the 16-byte read is skipped and there is no magic candidate.

Name and extension look at `ContentPath::as_bytes()`, not at the printable string. On Unix that includes non-UTF-8 names; see [ContentPath](content_path.md).

**Phase 2 — validate hits**, first success wins:

1. If there was a **magic** hit, call that identifier’s `validate`. `true` → that type.
2. Else (no magic hit, or `validate` returned `false`): if there was a **file-name** hit, `validate` it the same way.
3. Else: if there was an **extension** hit, `validate` it the same way.

**Phase 3 — custom identifiers**, only if phase 2 did not commit a type: every identifier with no `IdentifyMethod`, in registration order, `validate` only. The first `true` wins.

If nothing accepts, the object stays **unidentified**: no typed analyzers or extractors run; generic analyzers still do.

A magic hit that fails `validate` does **not** skip name and extension: those candidates were already computed, and they are validated next. Custom identifiers are **not** consulted while a fast-path candidate still might succeed.

```text
     content_type() already set? ──yes──► use it (identifiers skipped)
                │ no
                ▼
     ┌──────────────────────────────────────────────┐
     │ 1. Compute fast matches                      │
     │                                              │
     │    file name  →  type_from_file_name         │
     │    extension  →  type_from_extension         │
     │    if any Magic registered:                  │
     │        read(0, 16) → type_from_magic         │
     │    else: skip the 16-byte read               │
     └────────────────────┬─────────────────────────┘
                          │
                          ▼
     ┌──────────────────────────────────────────────┐
     │ 2. validate hits (first true wins)           │
     │                                              │
     │    magic hit?      ──validate──true──► type  │
     │    file-name hit?  ──validate──true──► type  │
     │    extension hit?  ──validate──true──► type  │
     └────────────────────┬─────────────────────────┘
                          │ no hit, or every validate was false
                          ▼
     ┌──────────────────────────────────────────────┐
     │ 3. Custom identifiers (no prefilter)         │
     │    validate only, registration order         │
     └────────────────────┬─────────────────────────┘
                          │ none accept
                          ▼
                        None (unidentified)
```

## `validate`

`validate` is called with `&mut dyn Content<T>` so you can `read` anywhere in the object. Use it to:

- Confirm a magic that is too short or too common (`MZ`, `PK`, `BM`).
- Inspect bytes **past** the 16-byte magic window (PE header, ZIP EOCD, PNG IHDR).
- Reject truncated or obviously corrupt files before analyzers run.
- Implement a fully custom identifier (`identify_method` returns `None`): entropy heuristic, JSON look-ahead, “this path is under `node_modules`,” and so on.

Returning `false` means “this is not my type,” not “abort the scan.” The scanner moves to the next candidate. Returning `true` **commits** the type for this object.

Keep `validate` cheaper than a full analysis. Heavy work (hashes, unpacking, deep parsing) belongs in an [analyzer](analyzer.md). Identifier code should answer yes/no with a few reads.

## Example: PE, magic `MZ`, then the PE header

A Windows Portable Executable starts with a DOS stub whose first two bytes are `MZ` (`0x4D 0x5A`). That is a good **fast** check: almost every PE has it, and matching two bytes on every file is cheap.

It is a bad **sufficient** check. DOS COM leftovers, some firmware blobs, and random data can start with `MZ`. A real PE also has a pointer at offset `0x3C` (`e_lfanew`) to a `PE\0\0` signature, then a COFF header whose **number of sections** is a `u16`. Zero sections is not a useful image. A huge count (the PE spec’s conventional ceiling is 96) is almost certainly not a PE either — it is truncated, random, or crafted to blow a parser.

So the identifier uses `Magic(b"MZ")` and puts the structural checks in `validate`:

```rust
struct PeIdentifier;

impl ContentIdentifier<MyTypes> for PeIdentifier {
    fn identify_method(&self) -> Option<IdentifyMethod> {
        Some(IdentifyMethod::Magic(b"MZ"))
    }

    fn validate(&self, content: &mut dyn Content<MyTypes>) -> bool {
        // DOS header: 64 bytes. e_lfanew (offset of the PE header) sits at 0x3C.
        let Some(dos) = content.read(0, 64) else {
            return false;
        };
        if dos.len() < 64 {
            return false;
        }
        let e_lfanew = u32::from_le_bytes(dos[0x3C..0x40].try_into().unwrap()) as u64;

        // "PE\0\0" (4 bytes) + COFF Machine (2) + NumberOfSections (2).
        let Some(pe) = content.read(e_lfanew, 8) else {
            return false;
        };
        if pe.len() < 8 || &pe[0..4] != b"PE\0\0" {
            return false;
        }
        let nsections = u16::from_le_bytes(pe[6..8].try_into().unwrap());

        // 0 is not an image; a huge count is not a sane PE either.
        (1..=96).contains(&nsections)
    }
}
```

Register it as usual — still one identifier for `MyTypes::Pe`:

```rust
ScannerBuilder::new()
    .add_identifier(MyTypes::Pe, PeIdentifier {})
    .add_analyzer(MyTypes::Pe, 0, PeAnalyzer {})
    .build();
```

What happens on a given file:

| Object | Fast check | `validate` | Result |
| --- | --- | --- | --- |
| `notepad.exe` | `MZ` matches | `PE\0\0`, sections in `1..=96` | typed as `Pe` |
| Random file starting `MZ..` | `MZ` matches | no PE signature, or 0 / 5000 sections | rejected; name/extension/custom tried next |
| `readme.txt` | no `MZ` | not called for `Pe` | other identifiers may still match |
| `BufferContent` already pinned as `Pe` | identifiers skipped | — | typed as `Pe` without this code |

The same pattern is how `ZipIdentifier` is written in the crate: fast `PK\x03\x04`, then `validate` searches the tail for an EOCD so a file that merely starts with those four bytes is not called a ZIP.

## Custom identifiers (`identify_method` → `None`)

If no `IdentifyMethod` fits — you need to look at byte 100, or at size, or at several disconnected fields — return `None` and do all of the work in `validate`. Those identifiers are **not** in the magic/name/extension matchers. They run **only in phase 3**: after the three fast matches have been computed and every hit has either been absent or failed `validate`. Until then they are not called. When they do run, it is in registration order, first `true` wins.

```rust
impl ContentIdentifier<MyTypes> for OddBlobIdentifier {
    fn identify_method(&self) -> Option<IdentifyMethod> {
        None
    }

    fn validate(&self, content: &mut dyn Content<MyTypes>) -> bool {
        content.size() > 16 && content.read(8, 4) == Some(b"ODDB")
    }
}
```

This is slower on a large tree (every still-untyped object calls your `validate`), so prefer a magic/name/extension pre-filter when you have one.

## Registration

```rust
.add_identifier(MyTypes::Pe, PeIdentifier {})
.add_identifier(MyTypes::Zip, ZipIdentifier::new())
```

Rules that show up as panics in `build()`:

- Two identifiers for the same `ContentType`.
- A `Magic` / `MultipleMagic` pattern longer than 16 bytes.

An object can have **no** identifier. Then it is only typed if something sets `content_type()` (a `FolderContent`, a child you constructed with a pinned type) or if a generic analyzer is enough and you do not need a type at all.

Identifiers classify. They do not write the [context](../chapter-4/context.md) or emit findings. Once a type is committed, [analyzers](analyzer.md) run.
