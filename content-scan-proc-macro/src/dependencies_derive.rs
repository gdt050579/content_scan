use proc_macro::{Delimiter, TokenStream, TokenTree};

pub(crate) fn process(input: TokenStream) -> Result<TokenStream, String> {
    let tokens: Vec<TokenTree> = input.into_iter().collect();
    let mut i = 0;

    let mut attr: Option<ParsedAttr> = None;

    while i < tokens.len() {
        match &tokens[i] {
            // Attribute: # [ ... ]
            TokenTree::Punct(p) if p.as_char() == '#' => {
                if let Some(TokenTree::Group(g)) = tokens.get(i + 1) {
                    if g.delimiter() == Delimiter::Bracket {
                        if let Some(parsed) = parse_dependencies_attr(g.stream())? {
                            if attr.is_some() {
                                return Err("multiple #[Dependencies(...)] attributes".to_string());
                            }
                            attr = Some(parsed);
                        }
                        i += 2;
                        continue;
                    }
                }
                i += 1;
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
            // The type keyword
            TokenTree::Ident(id) if matches!(id.to_string().as_str(), "struct" | "enum" | "union") => {
                i += 1;
                break;
            }
            TokenTree::Ident(id) => {
                return Err(format!("Dependencies can only be derived for structs or enums, found `{}`", id));
            }
            _ => i += 1,
        }
    }

    let type_name = match tokens.get(i) {
        Some(TokenTree::Ident(id)) => id.to_string(),
        _ => return Err("expected type name".to_string()),
    };
    i += 1;

    if let Some(TokenTree::Punct(p)) = tokens.get(i) {
        if p.as_char() == '<' {
            return Err("Dependencies does not support generic types".to_string());
        }
    }

    let attr = attr.ok_or_else(|| "Dependencies requires a #[Dependencies(name = \"...\")] attribute".to_string())?;

    let name_lit = match &attr.name {
        Some(lit) if string_lit_is_empty(lit) => {
            return Err("Dependencies `name` must not be empty".to_string());
        }
        Some(lit) => lit.clone(),
        None => return Err("Dependencies attribute is missing the `name` parameter".to_string()),
    };

    let deps = if attr.requires.is_empty() {
        "&[]".to_string()
    } else {
        format!("&[{}]", attr.requires.join(", "))
    };

    let out = format!(
        r#"
        impl Dependencies for {type_name} {{
            #[cfg(debug_assertions)]
            fn name(&self) -> &'static str {{
                {name_lit}
            }}
            #[cfg(debug_assertions)]
            fn dependencies(&self) -> &'static [&'static str] {{
                {deps}
            }}
        }}
        "#,
        type_name = type_name,
        name_lit = name_lit,
        deps = deps,
    );

    out.parse().map_err(|e| format!("failed to emit impl: {}", e))
}

struct ParsedAttr {
    name: Option<String>,
    requires: Vec<String>,
}

/// Parse a bracketed attribute. Returns `Ok(None)` if it is not `Dependencies(...)`.
fn parse_dependencies_attr(stream: TokenStream) -> Result<Option<ParsedAttr>, String> {
    let toks: Vec<TokenTree> = stream.into_iter().collect();
    if toks.is_empty() {
        return Ok(None);
    }
    let TokenTree::Ident(id) = &toks[0] else {
        return Ok(None);
    };
    if id.to_string() != "Dependencies" {
        return Ok(None);
    }

    let body = match toks.get(1) {
        Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis => g.stream(),
        Some(_) => return Err("expected `#[Dependencies(...)]`".to_string()),
        None => return Err("expected `#[Dependencies(...)]`".to_string()),
    };
    if toks.len() > 2 {
        return Err("unexpected tokens after #[Dependencies(...)]".to_string());
    }

    Ok(Some(parse_attr_body(body)?))
}

fn parse_attr_body(stream: TokenStream) -> Result<ParsedAttr, String> {
    let toks: Vec<TokenTree> = stream.into_iter().collect();
    let mut i = 0;
    let mut name: Option<String> = None;
    let mut requires: Option<Vec<String>> = None;

    while i < toks.len() {
        if let TokenTree::Punct(p) = &toks[i] {
            if p.as_char() == ',' {
                i += 1;
                continue;
            }
        }

        let key = match &toks[i] {
            TokenTree::Ident(id) => id.to_string(),
            other => return Err(format!("expected parameter name in Dependencies attribute, found `{}`", other)),
        };
        i += 1;

        match toks.get(i) {
            Some(TokenTree::Punct(p)) if p.as_char() == '=' => i += 1,
            _ => return Err(format!("expected `=` after `{key}` in Dependencies attribute")),
        }

        let value = toks.get(i).ok_or_else(|| format!("expected value after `{key} =` in Dependencies attribute"))?;

        match key.as_str() {
            "name" => {
                if name.is_some() {
                    return Err("duplicate `name` parameter in Dependencies attribute".to_string());
                }
                name = Some(expect_str_literal(value, "name")?);
                i += 1;
            }
            "requires" => {
                if requires.is_some() {
                    return Err("duplicate `requires` parameter in Dependencies attribute".to_string());
                }
                let list = parse_requires(value)?;
                i += 1;
                requires = Some(list);
            }
            other => {
                return Err(format!("unknown Dependencies parameter `{other}` (expected `name` or `requires`)"));
            }
        }
    }

    Ok(ParsedAttr {
        name,
        requires: requires.unwrap_or_default(),
    })
}

fn parse_requires(value: &TokenTree) -> Result<Vec<String>, String> {
    match value {
        TokenTree::Literal(_) => Ok(vec![expect_str_literal(value, "requires")?]),
        TokenTree::Group(g) if g.delimiter() == Delimiter::Bracket => {
            let inner: Vec<TokenTree> = g.stream().into_iter().collect();
            let mut items = Vec::new();
            let mut j = 0;
            while j < inner.len() {
                if let TokenTree::Punct(p) = &inner[j] {
                    if p.as_char() == ',' {
                        j += 1;
                        continue;
                    }
                }
                items.push(expect_str_literal(&inner[j], "requires")?);
                j += 1;
            }
            Ok(items)
        }
        other => Err(format!("Dependencies `requires` must be a string or an array of strings, found `{other}`")),
    }
}

fn expect_str_literal(tok: &TokenTree, param: &str) -> Result<String, String> {
    let TokenTree::Literal(lit) = tok else {
        return Err(format!("Dependencies `{param}` must be a string literal, found `{tok}`"));
    };
    let s = lit.to_string();
    if s.len() < 2 || !s.starts_with('"') || !s.ends_with('"') {
        return Err(format!("Dependencies `{param}` must be a string literal, found `{s}`"));
    }
    Ok(s)
}

fn string_lit_is_empty(lit: &str) -> bool {
    lit == "\"\""
}
