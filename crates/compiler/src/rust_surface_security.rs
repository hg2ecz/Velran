use crate::diagnostics::CompileError;

/// Fail-closed pre-parser guard for Rust-surface constructs that could escape the
/// Velran capability model if they ever reached generated Rust unchanged.
///
/// This is deliberately *not* a Rust type checker. rustc remains authoritative
/// for Rust syntax and typing. The guard owns only the security boundary:
/// ambient authority, FFI/ABI control, raw memory and compile-time file/env
/// access are rejected before any native build is attempted.
pub(crate) fn validate(source: &str) -> Result<(), CompileError> {
    let scrubbed = scrub_strings_and_comments(source);

    for (needle, label) in [
        ("unsafe", "`unsafe` blocks/functions/traits/impls"),
        ("extern", "`extern`/FFI declarations"),
        ("asm!", "inline assembly"),
        ("global_asm!", "global assembly"),
        ("include!", "compile-time source inclusion"),
        ("include_bytes!", "compile-time file reads"),
        ("include_str!", "compile-time file reads"),
        ("env!", "compile-time environment reads"),
        ("option_env!", "compile-time environment reads"),
        ("core::arch", "architecture intrinsics"),
        ("std::arch", "architecture intrinsics"),
        ("std::fs", "ambient filesystem access"),
        ("std::net", "ambient network access"),
        ("std::process", "process execution"),
        ("std::env", "ambient environment access"),
        ("std::thread", "unmanaged threads"),
        ("std::os", "OS-specific ambient authority"),
        ("std::ffi", "raw FFI access"),
        ("libc::", "libc/FFI access"),
        ("transmute", "raw memory reinterpretation"),
        ("from_raw", "raw ownership reconstruction"),
        ("into_raw", "raw ownership escape"),
        ("MaybeUninit", "uninitialized memory primitives"),
        ("ManuallyDrop", "manual drop control"),
        ("std::panic", "process-level panic control"),
        ("panic!", "panic/abort macros"),
        ("unreachable!", "panic/abort macros"),
        ("todo!", "panic/abort macros"),
        ("unimplemented!", "panic/abort macros"),
        ("assert!", "panic/abort assertions"),
        ("assert_eq!", "panic/abort assertions"),
        ("assert_ne!", "panic/abort assertions"),
    ] {
        if contains_security_token(&scrubbed, needle) {
            return Err(CompileError::security(
                "SEC-RUST-001",
                format!("Rust surface forbids {label}"),
                Some("use Velran typed values and explicit framework capabilities; rustc remains the type checker, but ambient authority and unsafe escape hatches are rejected before native compilation".to_owned()),
            ));
        }
    }

    // Raw pointers are not useful in the verified web surface and would make a
    // later passthrough feature too easy to turn into a capability bypass.
    if scrubbed.contains("*const ") || scrubbed.contains("*mut ") {
        return Err(CompileError::security(
            "SEC-RUST-001",
            "Rust surface forbids raw pointers".to_owned(),
            Some("use references, slices, Vec<T>, or framework-owned typed handles".to_owned()),
        ));
    }
    Ok(())
}

fn contains_security_token(source: &str, needle: &str) -> bool {
    if needle
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        source.match_indices(needle).any(|(start, _)| {
            let before = start.checked_sub(1).and_then(|i| source.as_bytes().get(i));
            let end = start + needle.len();
            let after = source.as_bytes().get(end);
            !before.is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
                && !after.is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
        })
    } else {
        source.contains(needle)
    }
}

fn scrub_strings_and_comments(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut out = String::with_capacity(source.len());
    let mut i = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    while i < bytes.len() {
        if in_string {
            let b = bytes[i];
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_string = false;
            }
            out.push(' ');
            i += 1;
            continue;
        }
        if bytes[i] == b'"' {
            in_string = true;
            out.push(' ');
            i += 1;
            continue;
        }
        if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'/') {
            while i < bytes.len() && bytes[i] != b'\n' {
                out.push(' ');
                i += 1;
            }
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_ambient_authority_and_unsafe_escape_hatches() {
        for source in [
            "fn x() { unsafe { } }",
            "fn x() { std::fs::read(\"x\"); }",
            "fn x() { std::process::Command::new(\"sh\"); }",
            "fn x() { let _ = include_str!(\"/etc/passwd\"); }",
            "extern \"C\" fn x() {}",
            "fn x(p: *const u8) {}",
            "fn x() { panic!(\"boom\"); }",
        ] {
            let err = validate(source).unwrap_err();
            assert!(err.to_string().contains("SEC-RUST-001"), "{source}: {err}");
        }
    }

    #[test]
    fn ignores_forbidden_words_inside_strings_and_comments() {
        validate("fn x() { let s = \"unsafe std::fs include_str!\"; } // extern std::net\n")
            .unwrap();
    }

    #[test]
    fn allows_safe_rust_like_compute_surface() {
        validate("fn x(values: &[String]) -> i64 { let n = values.len(); return n as i64; }")
            .unwrap();
    }
}
