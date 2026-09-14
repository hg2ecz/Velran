use super::CompileError;

pub(super) fn tokenize(source: &str) -> Result<Vec<String>, CompileError> {
    let c: Vec<char> = source.chars().collect();
    let mut o = Vec::new();
    let mut i = 0;
    while i < c.len() {
        if c[i].is_whitespace() {
            i += 1;
            continue;
        }
        if c[i] == '/' && i + 1 < c.len() && c[i + 1] == '/' {
            i += 2;
            while i < c.len() && c[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c[i] == '"' {
            i += 1;
            let mut s = String::new();
            let mut escaped = false;
            while i < c.len() {
                if escaped {
                    s.push(c[i]);
                    escaped = false;
                    i += 1;
                    continue;
                }
                if c[i] == '\\' {
                    escaped = true;
                    s.push(c[i]);
                    i += 1;
                    continue;
                }
                if c[i] == '"' {
                    break;
                }
                s.push(c[i]);
                i += 1;
            }
            if i >= c.len() {
                return Err(CompileError::Syntax("unterminated string".into()));
            }
            i += 1;
            o.push(s);
            continue;
        }
        if c[i] == '=' && i + 1 < c.len() && c[i + 1] == '>' {
            o.push("=>".into());
            i += 2;
            continue;
        }
        if c[i] == ';' {
            o.push(";".into());
            i += 1;
            continue;
        }
        // Signed decimal integer literals are required by range validation.
        // Keep the sign in the token so `-10` cannot silently become `10`.
        if (c[i] == '-' || c[i] == '+') && i + 1 < c.len() && c[i + 1].is_ascii_digit() {
            let s = i;
            i += 1;
            while i < c.len() && c[i].is_ascii_digit() {
                i += 1;
            }
            o.push(c[s..i].iter().collect());
            continue;
        }
        if c[i].is_ascii_alphanumeric() || c[i] == '_' || c[i] == '.' || c[i] == '<' || c[i] == '>'
        {
            let s = i;
            while i < c.len() {
                if c[i].is_ascii_alphanumeric()
                    || c[i] == '_'
                    || c[i] == '.'
                    || c[i] == '<'
                    || c[i] == '>'
                {
                    i += 1;
                    continue;
                }
                if c[i] == ':' && i + 1 < c.len() && c[i + 1] == ':' {
                    i += 2;
                    continue;
                }
                break;
            }
            o.push(c[s..i].iter().collect());
            continue;
        }
        if c[i] == '/' {
            let s = i;
            while i < c.len() && !c[i].is_whitespace() {
                i += 1;
            }
            o.push(c[s..i].iter().collect());
            continue;
        }

        let line = 1 + c[..i].iter().filter(|ch| **ch == '\n').count();
        let column = 1 + c[..i].iter().rev().take_while(|ch| **ch != '\n').count();
        return Err(CompileError::Syntax(format!(
            "unexpected character `{}` at line {line}, column {column}",
            c[i]
        )));
    }
    Ok(o)
}

use crate::expression_token::ExprToken;

pub(super) fn lex_expr(input: &str) -> Result<Vec<ExprToken>, CompileError> {
    let b = input.as_bytes();
    let mut o = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        match b[i] {
            b'&' if b.get(i + 1) == Some(&b'&') => {
                o.push(ExprToken::AndAnd);
                i += 2
            }
            b'|' if b.get(i + 1) == Some(&b'|') => {
                o.push(ExprToken::OrOr);
                i += 2
            }
            b'<' if b.get(i + 1) == Some(&b'<') => {
                o.push(ExprToken::ShiftLeft);
                i += 2
            }
            b'>' if b.get(i + 1) == Some(&b'>') => {
                o.push(ExprToken::ShiftRight);
                i += 2
            }
            b'<' if b.get(i + 1) == Some(&b'=') => {
                o.push(ExprToken::Le);
                i += 2
            }
            b'>' if b.get(i + 1) == Some(&b'=') => {
                o.push(ExprToken::Ge);
                i += 2
            }
            b'=' if b.get(i + 1) == Some(&b'=') => {
                o.push(ExprToken::EqEq);
                i += 2
            }
            b'!' if b.get(i + 1) == Some(&b'=') => {
                o.push(ExprToken::Ne);
                i += 2
            }
            b'<' => {
                o.push(ExprToken::Lt);
                i += 1
            }
            b'>' => {
                o.push(ExprToken::Gt);
                i += 1
            }
            b'[' => {
                o.push(ExprToken::LBracket);
                i += 1
            }
            b']' => {
                o.push(ExprToken::RBracket);
                i += 1
            }
            b',' => {
                o.push(ExprToken::Comma);
                i += 1
            }
            b';' => {
                o.push(ExprToken::Semi);
                i += 1
            }
            b'.' => {
                o.push(ExprToken::Dot);
                i += 1
            }
            b'+' => {
                o.push(ExprToken::Plus);
                i += 1
            }
            b'-' => {
                o.push(ExprToken::Minus);
                i += 1
            }
            b'*' => {
                o.push(ExprToken::Star);
                i += 1
            }
            b'/' => {
                o.push(ExprToken::Slash);
                i += 1
            }
            b'%' => {
                o.push(ExprToken::Percent);
                i += 1
            }
            b'&' => {
                o.push(ExprToken::Amp);
                i += 1
            }
            b'^' => {
                o.push(ExprToken::Caret);
                i += 1
            }
            b'|' => {
                o.push(ExprToken::Pipe);
                i += 1
            }
            b'!' => {
                o.push(ExprToken::Bang);
                i += 1
            }
            b'(' => {
                o.push(ExprToken::LParen);
                i += 1
            }
            b')' => {
                o.push(ExprToken::RParen);
                i += 1
            }
            b'"' => {
                i += 1;
                let mut s = String::new();
                while i < b.len() && b[i] != b'"' {
                    if b[i] == b'\\' {
                        i += 1;
                        let e = *b
                            .get(i)
                            .ok_or_else(|| CompileError::Syntax("unterminated escape".into()))?;
                        s.push(match e {
                            b'n' => '\n',
                            b'r' => '\r',
                            b't' => '\t',
                            b'"' => '"',
                            b'\\' => '\\',
                            x => x as char,
                        });
                        i += 1;
                    } else {
                        s.push(b[i] as char);
                        i += 1;
                    }
                }
                if i >= b.len() {
                    return Err(CompileError::Syntax("unterminated string".into()));
                }
                i += 1;
                o.push(ExprToken::String(s));
            }
            b'0'..=b'9' => {
                let start = i;
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
                let mut is_float = false;
                if i < b.len() && b[i] == b'.' && b.get(i + 1).is_some_and(|c| c.is_ascii_digit()) {
                    is_float = true;
                    i += 1;
                    while i < b.len() && b[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                if is_float {
                    if input.get(i..i + 3) != Some("f32") {
                        return Err(CompileError::Syntax(
                            "floating literal requires explicit `f32` suffix (example: 1.25f32)"
                                .into(),
                        ));
                    }
                    let raw = &input[start..i];
                    i += 3;
                    let value = raw
                        .parse::<f32>()
                        .ok()
                        .and_then(language_core::F32Value::new)
                        .ok_or_else(|| {
                            CompileError::Syntax("F32 literal must be finite and in range".into())
                        })?;
                    o.push(ExprToken::F32(value));
                } else {
                    o.push(ExprToken::Int(input[start..i].parse().map_err(|_| {
                        CompileError::Syntax("integer out of range".into())
                    })?));
                }
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                let s = i;
                i += 1;
                while i < b.len() {
                    if b[i].is_ascii_alphanumeric() || b[i] == b'_' {
                        i += 1;
                        continue;
                    }
                    if b[i] == b':' && b.get(i + 1) == Some(&b':') {
                        i += 2;
                        continue;
                    }
                    break;
                }
                o.push(ExprToken::Ident(input[s..i].into()));
            }
            x => {
                return Err(CompileError::Syntax(format!(
                    "unsupported expression char `{}`",
                    x as char
                )));
            }
        }
    }
    Ok(o)
}
