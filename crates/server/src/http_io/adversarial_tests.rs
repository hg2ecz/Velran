use super::parse_request_head;

#[test]
fn request_head_smuggling_corpus_fails_closed() {
    let bad = [
        "POST / HTTP/1.1\r\nHost: example\r\nContent-Length: 4\r\nContent-Length: 4",
        "POST / HTTP/1.1\r\nHost: example\r\nContent-Length: 4\r\nTransfer-Encoding: chunked",
        "POST / HTTP/1.1\r\nHost: example\r\nTransfer-Encoding: identity\r\nContent-Length: 4",
        "GET / HTTP/1.1\r\nHost: example\r\nContent-Length: 1",
        "POST / HTTP/1.1\r\nHost: example\r\nContent-Length: +4",
        "POST / HTTP/1.1\r\nHost: example\r\nContent-Length: 04x",
        "GET / HTTP/1.1\r\nHost: a\r\nHost: b",
        "GET / HTTP/1.1\r\nHost: example\r\n folded: x",
        "GET /%2e%2e/admin HTTP/1.1\r\nHost: example",
        "GET /a%2fb HTTP/1.1\r\nHost: example",
    ];
    for head in bad {
        assert!(
            parse_request_head(head, 64).is_err(),
            "smuggling/path corpus unexpectedly accepted: {head:?}"
        );
    }
}

#[test]
fn request_head_deterministic_mutation_corpus_never_panics() {
    const PIECES: &[&str] = &[
        "%",
        "%2e",
        "%2f",
        "%5c",
        "..",
        ".",
        "\\",
        "#",
        "\t",
        "\u{7f}",
        "Content-Length: 1\r\n",
        "Transfer-Encoding: chunked\r\n",
        "Host: b\r\n",
        "Connection: upgrade\r\n",
        "X-Test: a\r\n b\r\n",
    ];
    let base = "GET /safe HTTP/1.1\r\nHost: example";
    let mut state = 0x687474702026u64;
    for case in 0..2048usize {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let piece = PIECES[(state as usize) % PIECES.len()];
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut input = base.to_owned();
        let pos = (state as usize) % (input.len() + 1);
        if input.is_char_boundary(pos) {
            input.insert_str(pos, piece);
        }
        let result = std::panic::catch_unwind(|| parse_request_head(&input, 64));
        assert!(
            result.is_ok(),
            "HTTP parser panicked for adversarial case {case}: {input:?}"
        );
    }
}
