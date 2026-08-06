use proc_macro::{Delimiter, TokenStream, TokenTree};

pub(crate) fn process_content_type(input: TokenStream) -> Result<TokenStream, String> {
    let tokens: Vec<TokenTree> = input.into_iter().collect();
    let mut i = 0;

    let mut has_repr_u16 = false;

    while i < tokens.len() {
        match &tokens[i] {
            // Attribute: # [ ... ]
            TokenTree::Punct(p) if p.as_char() == '#' => {
                if let Some(TokenTree::Group(g)) = tokens.get(i + 1) {
                    if g.delimiter() == Delimiter::Bracket && is_repr_u16(g.stream()) {
                            has_repr_u16 = true;                        
                    }
                }
                i += 2;
            }
            // skip visibility
            TokenTree::Ident(id) if id.to_string() == "pub" => {
                i += 1;
                if let Some(TokenTree::Group(g)) = tokens.get(i) {
                    if g.delimiter() == Delimiter::Parenthesis {
                        i += 1;
                    }
                }
            }
            // The `enum` keyword
            TokenTree::Ident(id) if id.to_string() == "enum" => {
                i += 1;
                break;
            }
            // Anything else before `enum` — not supported (struct/union)
            TokenTree::Ident(id) => {
                return Err(format!("ContentType can only be derived for enums, found `{}`", id));
            }
            _ => i += 1,
        }
    }

    if !has_repr_u16 {
        return Err("ContentType requires #[repr(u16)] on the enum".to_string());
    }

    // Now expect: <Name> { <variants> }
    let name = match tokens.get(i) {
        Some(TokenTree::Ident(id)) => id.clone(),
        _ => return Err("expected enum name".to_string()),
    };
    i += 1;

    // Skip optional generics — not supported
    if let Some(TokenTree::Punct(p)) = tokens.get(i) {
        if p.as_char() == '<' {
            return Err("ContentType does not support generic enums".to_string());
        }
    }

    // Variant body
    let body = match tokens.get(i) {
        Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => g.stream(),
        _ => return Err("expected `{ ... }` variant body".to_string()),
    };

    let count = count_and_validate_variants(body)?;

    if count == 0 {
        return Err("ContentType requires at least one variant".to_string());
    }
    if count > u16::MAX as usize + 1 {
        return Err("ContentType enums must have at most 65536 variants".to_string());
    }

    let name_str = name.to_string();

    let out = format!(
        r#"
        impl ContentType for {name} {{
            const COUNT: u16 = {count};

            #[inline(always)]
            fn as_u16(&self) -> u16 {{
                *self as u16
            }}
            fn from_u16(value: u16) -> Option<Self> {{
                if value < {count} {{
                    Some(unsafe {{ std::mem::transmute(value) }})
                }} else {{
                    None
                }}
            }}
        }}
        "#,
        name = name_str,
        count = count,
    );

    out.parse().map_err(|e| format!("failed to emit impl: {}", e))
}

/// Check whether a bracketed attribute's contents are `repr(u16)`.
fn is_repr_u16(stream: TokenStream) -> bool {
    let toks: Vec<TokenTree> = stream.into_iter().collect();
    if toks.len() != 2 {
        return false;
    }
    let TokenTree::Ident(id) = &toks[0] else {
        return false;
    };
    if id.to_string() != "repr" {
        return false;
    }
    let TokenTree::Group(g) = &toks[1] else {
        return false;
    };
    if g.delimiter() != Delimiter::Parenthesis {
        return false;
    }
    let inner: Vec<TokenTree> = g.stream().into_iter().collect();
    if inner.len() != 1 {
        return false;
    }
    let TokenTree::Ident(id) = &inner[0] else {
        return false;
    };
    id.to_string() == "u16"
}

/// Walks the variants in the brace body, ensures each is a unit variant
/// with no explicit discriminant, and counts them.
fn count_and_validate_variants(stream: TokenStream) -> Result<usize, String> {
    let tokens: Vec<TokenTree> = stream.into_iter().collect();
    let mut count = 0usize;
    let mut i = 0;
    let mut just_saw_variant = false;

    while i < tokens.len() {
        match &tokens[i] {
            // Skip attributes on variants
            TokenTree::Punct(p) if p.as_char() == '#' => {
                if let Some(TokenTree::Group(g)) = tokens.get(i + 1) {
                    if g.delimiter() == Delimiter::Bracket {
                        i += 2;
                        continue;
                    }
                }
                i += 1;
            }

            // A variant identifier
            TokenTree::Ident(_) if !just_saw_variant => {
                count += 1;
                just_saw_variant = true;
                i += 1;
            }

            // Comma separates variants — reset
            TokenTree::Punct(p) if p.as_char() == ',' => {
                just_saw_variant = false;
                i += 1;
            }

            // Variant body — `Foo(x)` or `Foo { x }`
            TokenTree::Group(_) if just_saw_variant => {
                return Err("ContentType variants must be unit variants (no fields)".to_string());
            }

            // Discriminant — `Foo = 5`
            TokenTree::Punct(p) if p.as_char() == '=' && just_saw_variant => {
                return Err("ContentType variants cannot have explicit discriminants".to_string());
            }

            _ => i += 1,
        }
    }

    Ok(count)
}
