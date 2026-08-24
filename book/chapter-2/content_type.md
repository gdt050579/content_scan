# ContentType

Every `Scanner`, `Content`, identifier, analyzer, and extractor is parameterized by **one** enum: the kinds of content *this* program understands. That enum implements `ContentType`.

Two applications can depend on `content_scan` and still have unrelated type sets. There is no global catalog of “PNG” or “ZIP” inside the crate. `Png` is a variant you add; the ZIP built-ins are generic over *your* enum and you pass the variant you chose for archives.

```rust
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ContentType)]
#[repr(u16)]
enum MyTypes {
    Folder,
    Png,
    Zip,
    Text,
}
```

[Your first scan](../chapter-1/first_scan.md) used a single variant. A real scanner grows this list as you add formats.

## Why an enum, and why `u16`

The scanner’s dispatch tables are sized from `T::COUNT` and indexed by `T::as_u16()`. Identifiers, typed analyzers, and extractors are stored in those tables. A compact, stable integer per variant keeps lookup cheap.

That is also why there is **at most one identifier per variant**: the identifier table is a map from `u16` to one plugin. Analyzers and extractors are lists in the same slots, which is why you can register several of those per type. See [Architecture](architecture.md).

## Deriving it

`#[derive(ContentType)]` is re-exported from `content_scan`. The macro fills in the trait if the enum meets the contract:

| Requirement | Reason |
| --- | --- |
| It is an `enum` (not a struct) | Variants are the kinds. |
| `#[repr(u16)]` | `as_u16` is `*self as u16`. |
| Unit variants only | No `Png { .. }` or `Zip(u32)`. |
| No explicit discriminants | No `Png = 7`. Discriminants are 0, 1, 2, … in source order. |
| No generics | `enum Foo<T>` is rejected. |
| At least one variant, at most 65536 | `COUNT` fits in `u16`. |
| `Copy + Eq + Debug + Ord` | Required by the trait (derive them alongside `ContentType`). |

A typical header:

```rust
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ContentType)]
#[repr(u16)]
enum MyTypes {
    Folder,
    Png,
    Zip,
}
```

`Ord` / `PartialOrd` are used when the extractor list is sorted by type at `build()`. Skipping them is a compile error.

What the derive emits:

```rust
impl ContentType for MyTypes {
    const COUNT: u16 = 3; // number of variants

    fn as_u16(&self) -> u16 {
        *self as u16
    }

    fn from_u16(value: u16) -> Option<Self> {
        if value < 3 {
            Some(unsafe { std::mem::transmute(value) })
        } else {
            None
        }
    }
}
```

`COUNT` is the variant count, not “last discriminant + 1” from some other numbering scheme — because you are not allowed to set discriminants. Adding a variant at the **end** is the safe way to grow the enum. Inserting in the middle changes every later `u16` and would mismatch any stored ids; the scanner itself only uses these values in memory for the current process.

## The trait

You rarely implement `ContentType` by hand. The contract, if you do:

```rust
pub trait ContentType: Copy + Eq + PartialEq + Debug + Ord + PartialOrd {
    const COUNT: u16;
    fn as_u16(&self) -> u16;
    fn from_u16(value: u16) -> Option<Self>;
}
```

- `as_u16()` is unique per variant and **strictly less than** `COUNT`.
- `from_u16` is the inverse; unknown values return `None`.
- `COUNT` sizes the analyzer and extractor fast-maps.

## `bool`

`bool` implements `ContentType` (`false → 0`, `true → 1`, `COUNT = 2`). It is handy for tests and for a scanner that only needs two abstract kinds. Production tools almost always define their own enum.

## How the rest of the crate uses it

- **`ScannerBuilder<MyTypes>`** — every `add_identifier` / `add_analyzer` / `add_extractor` names a variant (generic analyzers are the exception: they are not keyed by type).
- **`Content<MyTypes>`** — `content_type()` returns `Option<MyTypes>`.
- **Identification** — a successful match becomes one variant, or the object stays unidentified (`None`).
- **Built-ins** — `ZipIdentifier`, `ZipExtractor`, and `FolderExtractor` are generic over `T`. You choose which variant means “ZIP” or “folder” when you register them.

A variant that you never register plugins for is still a valid type: you can pin it on a `BufferContent` and nothing typed will run, only generic analyzers. Unused variants still count toward `COUNT`.

[`ContentPath`](content_path.md) is independent of `ContentType`. The path names the object; this enum names the *kind*.
