use super::*;

#[cfg(test)]
mod resource_profile_config_tests {
    use super::*;

    #[test]
    fn parses_named_profiles_and_rejects_hard_ceiling_violation() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "velran-resource-profile-{}.toml",
            std::process::id()
        ));
        fs::write(
            &path,
            r#"
[profile.default]
max_instructions = 10
max_alloc_bytes = 100
max_concurrent = 8
[profile.compute]
max_instructions = 80
max_alloc_bytes = 900
max_concurrent = 2
"#,
        )
        .unwrap();
        let request = ExecutionLimits {
            max_instructions: 100,
            max_allocated_bytes: 1000,
            max_external_io_bytes: 1000,
        };
        let profiles = load_resource_profiles(Some(&path), &request, 8).unwrap();
        assert_eq!(profiles.default_config().max_instructions, 10);
        assert_eq!(profiles.config("compute").unwrap().max_concurrent, 2);

        fs::write(
            &path,
            r#"
[profile.compute]
max_instructions = 101
max_alloc_bytes = 900
max_concurrent = 2
"#,
        )
        .unwrap();
        assert!(load_resource_profiles(Some(&path), &request, 8).is_err());
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
mod m21_lifecycle_tests {
    use super::*;

    #[tokio::test]
    async fn liveness_and_dependency_free_readiness_are_cheap_and_no_store() {
        let lifecycle = LifecycleCliConfig::default();
        let live = serve_health_endpoint(&lifecycle.live_path, "GET", &lifecycle, None, None).await;
        assert_eq!(live.status, 200);
        assert_eq!(live.content_type(), "application/json; charset=utf-8");
        assert_eq!(live.body, br#"{"status":"live"}"#);
        assert!(
            live.headers()
                .any(|(k, v)| k == "Cache-Control" && v == "no-store")
        );

        let ready =
            serve_health_endpoint(&lifecycle.ready_path, "GET", &lifecycle, None, None).await;
        assert_eq!(ready.status, 200);
        assert_eq!(ready.body, br#"{"status":"ready"}"#);
    }

    #[tokio::test]
    async fn health_head_has_no_body_and_post_is_rejected() {
        let lifecycle = LifecycleCliConfig::default();
        let head =
            serve_health_endpoint(&lifecycle.live_path, "HEAD", &lifecycle, None, None).await;
        assert_eq!(head.status, 200);
        assert!(head.body.is_empty());
        let post =
            serve_health_endpoint(&lifecycle.live_path, "POST", &lifecycle, None, None).await;
        assert_eq!(post.status, 405);
    }

    #[test]
    fn validates_reserved_health_paths() {
        assert_eq!(
            validate_reserved_path("/internal/live").unwrap(),
            "/internal/live"
        );
        for bad in [
            "health/live",
            "//health",
            "/health/../x",
            "/health?x=1",
            "/health live",
        ] {
            assert!(validate_reserved_path(bad).is_err(), "{bad}");
        }
    }
}

#[cfg(test)]
mod server_config_file_tests {
    use super::*;

    #[test]
    fn config_rejects_unknown_and_plaintext_secret_keys() {
        assert!(toml::from_str::<ServerFileConfig>("[server]\nunknown = 1\n").is_err());
        assert!(
            toml::from_str::<ServerFileConfig>("[database]\nurl = 'postgres://secret'\n").is_err()
        );
    }

    #[test]
    fn config_rejects_duplicate_keys_and_relative_paths() {
        assert!(
            toml::from_str::<ServerFileConfig>(
                "[server]\nlisten='127.0.0.1:1'\nlisten='127.0.0.1:2'\n"
            )
            .is_err()
        );
        assert!(config_abs_path("relative/app.vrn", "server.app").is_err());
        assert!(config_abs_path("/srv/app/app.vrn", "server.app").is_ok());
    }

    #[test]
    fn sample_shape_parses() {
        let _: ServerFileConfig =
            toml::from_str(include_str!("../../../../config/server.toml.sample")).unwrap();
        let multi: ServerFileConfig = toml::from_str(include_str!(
            "../../../../config/server-multidomain.toml.sample"
        ))
        .unwrap();
        assert_eq!(multi.domains.len(), 2);
        assert_eq!(multi.domains[0].aliases.as_deref().unwrap().len(), 3);
        assert!(multi.domains[0].tls.cert_file.is_some());
        assert_eq!(multi.reload.enabled, Some(true));
        assert_eq!(multi.reload.mode.as_deref(), Some("development"));
        assert_eq!(multi.domains[0].reload.poll_interval_ms, Some(1000));
        let included: FileDomain = toml::from_str(include_str!(
            "../../../../config/domains/domain.toml.sample"
        ))
        .unwrap();
        assert_eq!(
            included.aliases.as_deref(),
            Some(&["control.example.com".to_string()][..])
        );
        assert!(included.tls.key_file.is_some());
        assert_eq!(included.reload.poll_interval_ms, Some(1500));
        let cfg: ServerFileConfig = toml::from_str(
            r#"
[server]
app = "/srv/app/app.vrn"
listen = "127.0.0.1:8080"
[limits]
max_connections = 100
[web]
cors_origins = ["https://example.com"]
"#,
        )
        .unwrap();
        assert_eq!(cfg.server.app.as_deref(), Some("/srv/app/app.vrn"));
        assert_eq!(cfg.limits.max_connections, Some(100));
        let rolling: ServerFileConfig = toml::from_str(include_str!(
            "../../../../config/server-rolling-prod.toml.sample"
        ))
        .unwrap();
        assert_eq!(rolling.reload.mode.as_deref(), Some("rolling"));
        assert_eq!(rolling.reload.debug_compile_errors, Some(false));
    }

    #[test]
    fn domain_config_rejects_global_process_limit_override() {
        assert!(
            toml::from_str::<FileDomain>(
                "host='a.example'\nworkdir='/srv/a'\n[limits]\nmax_process_memory_bytes=1\n"
            )
            .is_err()
        );
    }

    #[test]
    fn logging_config_is_strict_and_paths_are_absolute() {
        let cfg: ServerFileConfig = toml::from_str(
            r#"[logging]
server_file="/var/log/velran/server.log"
access_file="/var/log/velran/access.log"
audit_file="/var/log/velran/audit.log"
stderr=false
"#,
        )
        .unwrap();
        assert_eq!(cfg.logging.stderr, Some(false));
        let bad = LogConfig {
            server_file: Some(PathBuf::from("relative.log")),
            stderr: true,
            ..Default::default()
        };
        assert!(validate_log_config(&bad).is_err());
        assert!(toml::from_str::<ServerFileConfig>("[logging]\nunknown=true\n").is_err());
    }
}
