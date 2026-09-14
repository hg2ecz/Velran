use super::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_transfer_encoding() {
        let head = "POST / HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked";
        assert!(matches!(
            parse_request_head(head, 64),
            Err(HttpReadError::BadRequest)
        ));
    }

    #[test]
    fn rejects_compressed_request_bodies() {
        let compressed =
            "POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 4\r\nContent-Encoding: gzip";
        assert!(matches!(
            parse_request_head(compressed, 64),
            Err(HttpReadError::BadRequest)
        ));
        let identity =
            "POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 4\r\nContent-Encoding: identity";
        assert!(parse_request_head(identity, 64).is_ok());
    }

    #[test]
    fn parses_keep_alive_content_length() {
        let head = "POST /greet HTTP/1.1\r\nHost: localhost\r\nContent-Length: 9\r\nContent-Type: application/x-www-form-urlencoded";
        let parsed = parse_request_head(head, 64).unwrap();
        assert_eq!(parsed.content_length, 9);
        assert!(parsed.keep_alive);
    }

    #[test]
    fn connection_close_disables_keep_alive() {
        let head = "GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close";
        assert!(!parse_request_head(head, 64).unwrap().keep_alive);
    }
}

#[cfg(test)]
mod injection_security_tests {
    use super::*;

    #[test]
    fn rejects_obs_fold_and_non_ascii_headers() {
        let folded = "GET / HTTP/1.1\r\nHost: localhost\r\n x: y";
        assert!(matches!(
            parse_request_head(folded, 64),
            Err(HttpReadError::BadRequest)
        ));
        let non_ascii = "GET / HTTP/1.1\r\nHost: localhöst";
        assert!(matches!(
            parse_request_head(non_ascii, 64),
            Err(HttpReadError::BadRequest)
        ));
    }

    #[test]
    fn rejects_upgrade_expect_and_proxy_connection() {
        for header in [
            "Upgrade: websocket",
            "Expect: 100-continue",
            "Proxy-Connection: keep-alive",
            "Trailer: x",
        ] {
            let head = format!("GET / HTTP/1.1\r\nHost: localhost\r\n{header}");
            assert!(matches!(
                parse_request_head(&head, 64),
                Err(HttpReadError::BadRequest)
            ));
        }
    }

    #[test]
    fn enforces_header_count_limit() {
        let head = "GET / HTTP/1.1\r\nHost: localhost\r\nA: 1";
        assert!(matches!(
            parse_request_head(head, 1),
            Err(HttpReadError::HeaderTooLarge)
        ));
    }

    #[test]
    fn rejects_duplicate_content_type_and_ambiguous_connection_tokens() {
        let dup = "POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 1\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Type: text/plain";
        assert!(matches!(
            parse_request_head(dup, 64),
            Err(HttpReadError::BadRequest)
        ));
        let hop = "GET / HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive, transfer-encoding";
        assert!(matches!(
            parse_request_head(hop, 64),
            Err(HttpReadError::BadRequest)
        ));
    }

    #[test]
    fn rejects_duplicate_forwarded_proto() {
        let head = "GET / HTTP/1.1\r\nHost: localhost\r\nX-Forwarded-Proto: https\r\nX-Forwarded-Proto: https";
        assert!(matches!(
            parse_request_head(head, 64),
            Err(HttpReadError::BadRequest)
        ));
    }

    #[test]
    fn rejects_get_body_and_zero_length_post() {
        let get = "GET / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 1";
        assert!(matches!(
            parse_request_head(get, 64),
            Err(HttpReadError::BadRequest)
        ));
        let post = "POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0";
        assert!(matches!(
            parse_request_head(post, 64),
            Err(HttpReadError::BadRequest)
        ));
    }
}

#[cfg(test)]
mod web_security_tests {
    use super::*;
    fn req(headers: Vec<(&str, &str)>) -> HttpRequest {
        HttpRequest {
            method: "POST".into(),
            target: "/x".into(),
            headers: headers
                .into_iter()
                .map(|(a, b)| (a.into(), b.into()))
                .collect(),
            body: Vec::new(),
        }
    }
    #[test]
    fn rejects_cross_site_fetch_metadata() {
        let r = req(vec![
            ("host", "example.com"),
            ("sec-fetch-site", "cross-site"),
            ("origin", "https://example.com"),
        ]);
        assert!(
            validate_browser_state_change(
                &r,
                true,
                Some("example.com"),
                &WebSecurityCliConfig::default()
            )
            .is_err()
        );
    }
    #[test]
    fn exact_origin_is_required() {
        let good = req(vec![
            ("host", "example.com"),
            ("origin", "https://example.com"),
        ]);
        assert!(
            validate_browser_state_change(
                &good,
                true,
                Some("example.com"),
                &WebSecurityCliConfig::default()
            )
            .is_ok()
        );
        let bad = req(vec![
            ("host", "example.com"),
            ("origin", "https://evil.example"),
        ]);
        assert!(
            validate_browser_state_change(
                &bad,
                true,
                Some("example.com"),
                &WebSecurityCliConfig::default()
            )
            .is_err()
        );
    }
    #[test]
    fn forwarded_headers_only_from_trusted_proxy() {
        let r = req(vec![
            ("host", "example.com"),
            ("x-forwarded-for", "203.0.113.8"),
        ]);
        let peer: IpAddr = "127.0.0.1".parse().unwrap();
        assert!(effective_client_ip(&r, peer, &[]).is_err());
        let net: IpNet = "127.0.0.0/8".parse().unwrap();
        assert_eq!(
            effective_client_ip(&r, peer, &[net]).unwrap(),
            "203.0.113.8".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn x_real_ip_is_supported_only_from_trusted_proxy() {
        let r = req(vec![("x-real-ip", "198.51.100.9")]);
        let peer: IpAddr = "127.0.0.1".parse().unwrap();
        assert!(effective_client_ip(&r, peer, &[]).is_err());
        let net: IpNet = "127.0.0.0/8".parse().unwrap();
        assert_eq!(
            effective_client_ip(&r, peer, &[net]).unwrap(),
            "198.51.100.9".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn multiple_client_forwarding_headers_are_rejected() {
        let r = req(vec![
            ("x-real-ip", "198.51.100.9"),
            ("x-forwarded-for", "198.51.100.9"),
        ]);
        let peer: IpAddr = "127.0.0.1".parse().unwrap();
        let net: IpNet = "127.0.0.0/8".parse().unwrap();
        assert!(effective_client_ip(&r, peer, &[net]).is_err());
    }

    #[test]
    fn forwarded_https_only_from_trusted_proxy() {
        let r = req(vec![
            ("host", "example.com"),
            ("x-forwarded-proto", "https"),
        ]);
        let peer: IpAddr = "127.0.0.1".parse().unwrap();
        assert!(effective_request_https(&r, peer, false, &[]).is_err());
        let net: IpNet = "127.0.0.0/8".parse().unwrap();
        assert_eq!(effective_request_https(&r, peer, false, &[net]), Ok(true));
    }

    #[test]
    fn forwarded_proto_rejects_ambiguous_or_invalid_values() {
        let peer: IpAddr = "127.0.0.1".parse().unwrap();
        let net: IpNet = "127.0.0.0/8".parse().unwrap();
        let ambiguous = req(vec![
            ("forwarded", "for=203.0.113.8;proto=https"),
            ("x-forwarded-proto", "https"),
        ]);
        assert!(effective_request_https(&ambiguous, peer, false, &[net]).is_err());
        let invalid = req(vec![("x-forwarded-proto", "ftp")]);
        assert!(effective_request_https(&invalid, peer, false, &[net]).is_err());
    }
}

#[cfg(test)]
mod tls_security_tests {
    use super::*;
    use crate::operations::safe_redirect_location;
    #[test]
    fn public_host_validation_rejects_injection() {
        assert!(validate_public_host("example.com").is_ok());
        assert!(validate_public_host("evil.com/path").is_err());
        assert!(validate_public_host("evil.com\r\nX: y").is_err());
        assert!(validate_public_host("user@evil.com").is_err());
    }
    #[test]
    fn redirect_location_rejects_crlf() {
        assert!(safe_redirect_location("https://example.com/a"));
        assert!(!safe_redirect_location("https://example.com/a\r\nX: y"));
    }
    #[test]
    fn host_header_can_be_pinned_to_public_host() {
        assert!(host_matches_public(Some("example.com"), "example.com"));
        assert!(host_matches_public(Some("example.com:443"), "example.com"));
        assert!(!host_matches_public(Some("evil.example"), "example.com"));
        assert!(!host_matches_public(
            Some("example.com.evil"),
            "example.com"
        ));
    }
}

#[cfg(test)]
mod m19_json_cors_tests {
    use super::*;
    use compiler::compile_source;

    fn request(method: &str, target: &str, headers: Vec<(&str, &str)>, body: &[u8]) -> HttpRequest {
        HttpRequest {
            method: method.into(),
            target: target.into(),
            headers: headers
                .into_iter()
                .map(|(a, b)| (a.into(), b.into()))
                .collect(),
            body: body.to_vec(),
        }
    }

    #[test]
    fn strict_json_rejects_duplicates_nested_and_floats() {
        assert!(
            decode_json_object_limited(br#"{"name":"A","age":42,"active":true}"#, 8, 128).is_ok()
        );
        assert!(decode_json_object_limited(br#"{"name":"A","name":"B"}"#, 8, 128).is_err());
        assert!(decode_json_object_limited(br#"{"x":{"nested":1}}"#, 8, 128).is_err());
        assert!(decode_json_object_limited(br#"{"x":1.5}"#, 8, 128).is_err());
    }

    #[test]
    fn content_negotiation_honors_accept() {
        assert!(accepts_media(Some("application/json"), "application/json"));
        assert!(accepts_media(Some("application/*"), "application/json"));
        assert!(accepts_media(Some("*/*"), "text/html"));
        assert!(!accepts_media(Some("text/html"), "application/json"));
        assert!(!accepts_media(
            Some("application/json;q=0"),
            "application/json"
        ));
    }

    #[test]
    fn cors_preflight_is_origin_and_route_scoped() {
        let program = compile_source(
            r#"
#[page] fn api(ctx: PageContext) -> Result<Json, PageError> {
    let ok = true;
    return Ok(json(ok));
}
route api GET "/api" public => api;
"#,
        )
        .unwrap();
        let mut web = WebSecurityCliConfig::default();
        web.cors_origins.push("https://frontend.example".into());
        let ok = request(
            "OPTIONS",
            "/api",
            vec![
                ("origin", "https://frontend.example"),
                ("access-control-request-method", "GET"),
            ],
            b"",
        );
        assert_eq!(cors_preflight(&ok, &web, &program).status, 204);
        let denied = request(
            "OPTIONS",
            "/api",
            vec![
                ("origin", "https://evil.example"),
                ("access-control-request-method", "GET"),
            ],
            b"",
        );
        assert_eq!(cors_preflight(&denied, &web, &program).status, 403);
        let missing = request(
            "OPTIONS",
            "/missing",
            vec![
                ("origin", "https://frontend.example"),
                ("access-control-request-method", "GET"),
            ],
            b"",
        );
        assert_eq!(cors_preflight(&missing, &web, &program).status, 404);
    }

    #[test]
    fn credentialed_cors_allows_only_configured_origin_for_state_change() {
        let mut web = WebSecurityCliConfig::default();
        web.cors_origins.push("https://frontend.example".into());
        web.cors_allow_credentials = true;
        let good = request(
            "POST",
            "/api",
            vec![
                ("origin", "https://frontend.example"),
                ("sec-fetch-site", "cross-site"),
            ],
            b"{}",
        );
        assert!(validate_browser_state_change(&good, true, Some("api.example"), &web).is_ok());
        let bad = request(
            "POST",
            "/api",
            vec![
                ("origin", "https://evil.example"),
                ("sec-fetch-site", "cross-site"),
            ],
            b"{}",
        );
        assert!(validate_browser_state_change(&bad, true, Some("api.example"), &web).is_err());
    }
}

#[cfg(test)]
mod m20_static_asset_tests {
    use super::*;

    #[test]
    fn fingerprint_detection_requires_hex_token() {
        assert!(fingerprinted_asset("app.a1b2c3d4.js"));
        assert!(fingerprinted_asset("css/site-0123456789abcdef.css"));
        assert!(!fingerprinted_asset("app.production.js"));
        assert!(!fingerprinted_asset("app.deadbeeg.js"));
    }

    #[test]
    fn etag_and_if_none_match_are_representation_specific() {
        let a = static_etag(b"abc");
        let b = static_etag(b"abcd");
        assert_ne!(a, b);
        assert!(if_none_match(Some(&a), &a));
        assert!(if_none_match(Some(&format!("W/{a}")), &a));
        assert!(if_none_match(Some("*"), &a));
    }

    #[test]
    fn accept_encoding_respects_explicit_zero_quality() {
        assert!(encoding_accepted("br, gzip;q=0.5", "br"));
        assert!(!encoding_accepted("br;q=0, gzip", "br"));
        assert!(encoding_accepted("gzip", "gzip"));
    }

    #[test]
    fn static_path_is_deliberately_strict() {
        assert!(valid_static_relative("css/app.a1b2c3d4.css"));
        assert!(!valid_static_relative("../secret"));
        assert!(!valid_static_relative("css/%2e%2e/secret"));
        assert!(!valid_static_relative("css/app file.css"));
        assert!(!valid_static_relative("/css/app.css"));
    }

    #[tokio::test]
    async fn static_asset_get_head_etag_and_precompressed() {
        let root = std::env::temp_dir().join(format!(
            "velran-static-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("js")).unwrap();
        fs::write(root.join("js/app.a1b2c3d4.js"), b"console.log('plain');").unwrap();
        fs::write(root.join("js/app.a1b2c3d4.js.br"), b"brotli-bytes").unwrap();
        let assets = StaticAssets {
            fs: AppFs::open_root(
                &root,
                FsMode {
                    read: true,
                    write: false,
                    create: false,
                },
                FsLimits {
                    max_file_bytes: 1024,
                    ..FsLimits::default()
                },
            )
            .unwrap(),
            url_prefix: "/assets/".into(),
            regular_max_age_secs: 300,
            immutable_max_age_secs: 31536000,
            precompressed: true,
        };
        let req = HttpRequest {
            method: "GET".into(),
            target: "/assets/js/app.a1b2c3d4.js".into(),
            headers: vec![("accept-encoding".into(), "br".into())],
            body: vec![],
        };
        let r = serve_static_asset(&assets, &req, "/assets/js/app.a1b2c3d4.js").await;
        assert_eq!(r.status, 200);
        assert_eq!(r.body, b"brotli-bytes");
        assert!(
            r.headers()
                .any(|(k, v)| k == "Content-Encoding" && v == "br")
        );
        assert!(
            r.headers()
                .any(|(k, v)| k == "Cache-Control" && v.contains("immutable"))
        );
        let etag = r
            .headers()
            .find(|(k, _)| *k == "ETag")
            .unwrap()
            .1
            .to_string();

        let req304 = HttpRequest {
            method: "GET".into(),
            target: "/assets/js/app.a1b2c3d4.js".into(),
            headers: vec![
                ("accept-encoding".into(), "br".into()),
                ("if-none-match".into(), etag),
            ],
            body: vec![],
        };
        assert_eq!(
            serve_static_asset(&assets, &req304, "/assets/js/app.a1b2c3d4.js")
                .await
                .status,
            304
        );

        let head = HttpRequest {
            method: "HEAD".into(),
            target: "/assets/js/app.a1b2c3d4.js".into(),
            headers: vec![],
            body: vec![],
        };
        let hr = serve_static_asset(&assets, &head, "/assets/js/app.a1b2c3d4.js").await;
        assert!(hr.suppress_body);
        assert_eq!(
            hr.content_length_override,
            Some(b"console.log('plain');".len())
        );
        let _ = fs::remove_dir_all(root);
    }
}

#[test]
fn strict_json_accepts_bounded_string_arrays_and_rejects_unsafe_arrays() {
    let pairs = decode_json_object_limited(br#"{"tags":["rust","web"]}"#, 8, 128).unwrap();
    assert_eq!(
        pairs,
        vec![
            ("tags".to_string(), "rust".to_string()),
            ("tags".to_string(), "web".to_string()),
        ]
    );
    assert!(decode_json_object_limited(br#"{"tags":[]}"#, 8, 128).is_err());
    assert!(decode_json_object_limited(br#"{"tags":["ok",1]}"#, 8, 128).is_err());
    assert!(decode_json_object_limited(br#"{"tags":["a","b","c"]}"#, 2, 128).is_err());
}

#[test]
fn bounded_json_rejects_duplicate_nested_keys_and_resource_excess() {
    use crate::request_json::{JsonLimits, decode_json_object_bounded};
    let limits = JsonLimits {
        max_depth: 2,
        max_string_bytes: 16,
        max_array_items: 2,
        max_object_fields: 3,
    };
    assert!(decode_json_object_bounded(br#"{"name":"ok"}"#, 8, 16, limits).is_ok());
    assert!(decode_json_object_bounded(br#"{"x":{"a":1,"a":2}}"#, 8, 16, limits).is_err());
    assert!(decode_json_object_bounded(br#"{"x":{"y":{"z":1}}}"#, 8, 16, limits).is_err());
    assert!(decode_json_object_bounded(br#"{"tags":["a","b","c"]}"#, 8, 16, limits).is_err());
    assert!(decode_json_object_bounded(br#"{"name":"0123456789abcdefx"}"#, 8, 32, limits).is_err());
}
