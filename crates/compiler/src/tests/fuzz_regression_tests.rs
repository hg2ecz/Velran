use super::*;

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

fn mutate(seed: &str, state: &mut u64) -> String {
    const TOKENS: &[&str] = &[
        "",
        "{",
        "}",
        "(",
        ")",
        "[",
        "]",
        "<",
        ">",
        "::",
        ".",
        "..",
        "?",
        "%",
        "\\0",
        "\"",
        "'",
        "/*",
        "*/",
        "//",
        "Option<",
        "Result<",
        "Json<",
        "Query<",
        "Form<",
        "Multipart<",
        ".unwrap_or(",
        ".is_some()",
        "return",
        "while",
        "unsafe",
        "extern",
    ];
    let mut out = seed.to_owned();
    let operations = (lcg(state) % 8 + 1) as usize;
    for _ in 0..operations {
        let position = (lcg(state) as usize) % (out.len() + 1);
        let token = TOKENS[(lcg(state) as usize) % TOKENS.len()];
        match lcg(state) % 3 {
            0 => out.insert_str(position, token),
            1 if !out.is_empty() => {
                let end = (position + 1).min(out.len());
                if out.is_char_boundary(position) && out.is_char_boundary(end) && position < end {
                    out.replace_range(position..end, token);
                }
            }
            _ if !out.is_empty() => {
                let end = (position + ((lcg(state) % 7) as usize)).min(out.len());
                if out.is_char_boundary(position) && out.is_char_boundary(end) && position < end {
                    out.replace_range(position..end, "");
                }
            }
            _ => {}
        }
        if out.len() > 16 * 1024 {
            out.truncate(16 * 1024);
        }
    }
    out
}

#[test]
fn deterministic_parser_verifier_mutation_corpus_never_panics() {
    let seeds = [
        "#[page]\nfn home(ctx: PageContext) -> Result<Html, PageError> { Ok(html {<p>Hello</p>}) }\nroute home GET \"/\" public => home;",
        "struct Input { q: String, page: i64 }\n#[page]\nfn search(ctx: PageContext, input: Query<Input>) -> Result<Html, PageError> { let q = input.q.trim(); Ok(html {<p>{{ q }}</p>}) }\nroute search GET \"/search\" query Input public => search;",
        "fn maybe(input: &str) -> Option<String> { return Some(input.trim().to_lowercase()); }\n#[page]\nfn p(ctx: PageContext, text: String) -> Result<Html, PageError> { let v = maybe(&text); let s = v.unwrap_or(\"x\"); Ok(html {<p>{{ s }}</p>}) }\nroute p GET \"/p/:text<String>\" public => p;",
        "struct UploadInput { title: String, file: Upload }\n#[action]\nfn save(ctx: ActionContext, input: Multipart<UploadInput>) -> Result<Json, PageError> { return Ok(json(input.title)); }\nroute save POST \"/upload\" multipart UploadInput to \"private\" public => save;",
    ];
    let mut state = 0x72_77_6c_61_6e_67_2026u64;
    for case in 0..1024usize {
        let source = mutate(seeds[case % seeds.len()], &mut state);
        let result = std::panic::catch_unwind(|| crate::compile_verified_source(&source));
        assert!(
            result.is_ok(),
            "compiler panicked for deterministic mutation case {case}: {source:?}"
        );
    }
}
