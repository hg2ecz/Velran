use super::*;

#[cfg(test)]
mod m22_route_rate_limit_tests {
    use super::*;

    #[tokio::test]
    async fn memory_fixed_window_limits_by_ip_route() {
        let mut policies = HashMap::new();
        policies.insert(
            "api".into(),
            RatePolicy {
                limit: 2,
                window_secs: 60,
                scope: RateScope::IpRoute,
            },
        );
        let limiter = RouteRateLimiter {
            policies: Arc::new(policies),
            redis: None,
            memory: Arc::new(Mutex::new(HashMap::new())),
        };
        assert!(
            limiter
                .check("api", "home", "203.0.113.9", None)
                .await
                .unwrap()
                .0
        );
        assert!(
            limiter
                .check("api", "home", "203.0.113.9", None)
                .await
                .unwrap()
                .0
        );
        assert!(
            !limiter
                .check("api", "home", "203.0.113.9", None)
                .await
                .unwrap()
                .0
        );
        assert!(
            limiter
                .check("api", "other", "203.0.113.9", None)
                .await
                .unwrap()
                .0
        );
    }

    #[test]
    fn public_route_cannot_use_user_scope() {
        let program = compiler::compile_source(
            r#"
#[page] fn home(ctx: PageContext) -> Result<Html, PageError> {
    return Ok(html {ok});
}
route home GET "/" public rate perUser => home;
"#,
        )
        .unwrap();
        let mut policies = HashMap::new();
        policies.insert(
            "perUser".into(),
            RatePolicy {
                limit: 10,
                window_secs: 60,
                scope: RateScope::User,
            },
        );
        assert!(validate_route_rate_policies(&program, &policies).is_err());
    }
}

#[cfg(test)]
mod m23_observability_tests {
    use super::*;

    #[test]
    fn response_observation_adds_request_id_and_metrics() {
        let metrics = Metrics::default();
        let timer = RequestTimer::start();
        let mut response = Response::text(200, "OK", b"ok\n");
        observe_response(
            &mut response,
            "velran-test",
            "GET",
            "home",
            "127.0.0.1",
            0,
            &timer,
            &metrics,
            false,
        );
        assert_eq!(
            response
                .headers()
                .find(|(n, _)| n.eq_ignore_ascii_case("x-request-id"))
                .map(|(_, v)| v),
            Some("velran-test")
        );
        let text = metrics.render_prometheus();
        assert!(text.contains("velran_requests_total 1"));
        assert!(text.contains("velran_route_requests_total{route=\"home\"} 1"));
    }

    #[test]
    fn security_audit_classifies_trust_boundary_denials() {
        assert_eq!(
            security_audit_classification(400, b"invalid forwarding headers\n"),
            Some(("proxy", "invalid_forwarding"))
        );
        assert_eq!(
            security_audit_classification(403, b"CSRF validation failed\n"),
            Some(("csrf", "denied"))
        );
        assert_eq!(
            security_audit_classification(403, b"CORS origin denied\n"),
            Some(("cors", "denied"))
        );
        assert_eq!(
            security_audit_classification(403, b"origin validation failed\n"),
            Some(("origin", "denied"))
        );
        assert_eq!(
            security_audit_classification(421, b"unexpected host\n"),
            Some(("host", "mismatch"))
        );
        assert_eq!(
            security_audit_classification(426, b"HTTPS required\n"),
            Some(("transport", "https_required"))
        );
        assert_eq!(security_audit_classification(404, b"not found\n"), None);
    }

    #[test]
    fn public_metrics_requires_explicit_escape_hatch() {
        let public = "0.0.0.0:9090".parse::<std::net::SocketAddr>().unwrap();
        assert!(!public.ip().is_loopback());
        let local = "127.0.0.1:9090".parse::<std::net::SocketAddr>().unwrap();
        assert!(local.ip().is_loopback());
    }
}
