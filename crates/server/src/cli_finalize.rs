use crate::bootstrap_config::{
    load_rate_policies, load_resource_profiles, print_effective_config,
    validate_route_rate_policies,
};
use crate::cli_config_apply::LoadedCliConfig;
use crate::cli_overrides::AppliedCli;
use crate::cli_scan::CliBootstrap;
use crate::server_config_file::{build_domain_configs, validate_log_config};
use crate::server_errors::CliParseError;
use crate::startup_args::StartupArgs;
use crate::static_delivery::validate_static_prefix;
use crate::tls_support::build_tls_acceptor;
use compiler::compile_file;
use data::DbConfig;
use runtime::ExecutionLimits;
use std::fs;
use std::path::PathBuf;
pub(super) fn finalize(
    applied: AppliedCli,
    bootstrap: CliBootstrap,
) -> Result<StartupArgs, CliParseError> {
    let AppliedCli {
        loaded,
        force_disable_source_reload,
    } = applied;
    let LoadedCliConfig {
        app,
        config,
        db_url,
        resource_profiles_file,
        domain_entries,
        web,
        storage,
        mut static_assets,
        lifecycle,
        rate_limits_file,
        allow_memory_rate_limit,
        observability,
        log_config,
        cache_cli,
        native,
        source_reload,
        resource_limits,
        allow_insecure_db,
        auth,
        tls,
        unix_socket,
        behind_proxy,
    } = loaded;
    let CliBootstrap {
        config_path,
        check_config,
        print_effective,
    } = bootstrap;
    if auth.login_max_attempts == 0
        || auth.login_principal_max_attempts == 0
        || auth.login_source_max_attempts == 0
        || auth.mfa_max_attempts == 0
        || auth.login_window_secs == 0
    {
        return Err("authentication abuse limits must be greater than zero".into());
    }
    if tls.handshake_timeout_ms == 0 {
        return Err("--tls-handshake-timeout-ms must be greater than zero".into());
    }
    if tls.cert_file.is_some() != tls.key_file.is_some() {
        return Err("--tls-cert-file and --tls-key-file must be supplied together".into());
    }
    if web.cors_allow_credentials && web.cors_origins.is_empty() {
        return Err("--cors-allow-credentials requires at least one --cors-origin".into());
    }
    if web.cors_allow_credentials && config.insecure_dev_cookies {
        return Err(
            "credentialed CORS requires HTTPS; it cannot be combined with --insecure-dev-cookies"
                .into(),
        );
    }
    let db_configured = db_url.is_some();
    let db_config = db_url.map(|url| {
        let mut c = DbConfig::secure_default(url);
        if allow_insecure_db {
            c.require_tls_for_remote = false;
        }
        c
    });
    if storage.max_upload_bytes == 0 {
        return Err("--max-upload-bytes must be greater than zero".into());
    }
    if storage.max_image_pixels == 0 || storage.max_image_pixels > 400_000_000 {
        return Err("--max-image-pixels must be between 1 and 400000000".into());
    }
    if static_assets.max_asset_bytes == 0 {
        return Err("--max-static-asset-bytes must be greater than zero".into());
    }
    static_assets.url_prefix = validate_static_prefix(&static_assets.url_prefix)?;
    if lifecycle.live_path == lifecycle.ready_path {
        return Err("liveness and readiness paths must differ".into());
    }
    if lifecycle.dependency_timeout_ms == 0 || lifecycle.shutdown_grace_ms == 0 {
        return Err(
            "health dependency timeout and shutdown grace must be greater than zero".into(),
        );
    }
    if cache_cli.singleflight_wait_timeout_ms == 0 {
        return Err("cache single-flight wait timeout must be greater than zero".into());
    }
    if cache_cli.max_ttl_secs == 0 || cache_cli.max_ttl_secs > 604800 {
        return Err("--cache-max-ttl-secs must be between 1 and 604800".into());
    }
    if cache_cli.max_entries == 0 || cache_cli.max_bytes == 0 {
        return Err("cache entry/byte limits must be greater than zero".into());
    }
    if config.max_header_bytes == 0
        || config.max_body_bytes == 0
        || config.max_connections == 0
        || config.max_requests_per_connection == 0
        || config.max_header_count == 0
        || config.max_form_fields == 0
        || config.max_form_field_bytes == 0
        || [
            config.max_json_depth,
            config.max_json_string_bytes,
            config.max_json_array_items,
            config.max_json_object_fields,
        ]
        .contains(&0)
        || config.max_instructions == 0
        || config.max_runtime_alloc_bytes == 0
        || config.session_ttl_secs == 0
        || config.max_sessions == 0
    {
        return Err("server limits must be greater than zero".into());
    }
    if source_reload.poll_interval_ms == 0 || source_reload.debounce_ms == 0 {
        return Err("source reload poll/debounce intervals must be greater than zero".into());
    }
    if !domain_entries.is_empty() && (storage.data_root.is_some() || static_assets.root.is_some()) {
        return Err("multi-domain mode requires storage.data_root/static_assets.root to be configured per domain, relative to workdir".into());
    }
    let mut domains = build_domain_configs(
        domain_entries,
        &config,
        &storage,
        &static_assets,
        resource_profiles_file.as_deref(),
        &source_reload,
    )?;
    if force_disable_source_reload {
        for domain in &mut domains {
            domain.reload.enabled = false;
        }
    }
    if !domains.is_empty() && app.is_some() {
        return Err("`server.app`/--app cannot be combined with [[domains]]".into());
    }
    let app = if domains.is_empty() {
        app.ok_or(
            "missing application: configure `server.app`, --app, or at least one [[domains]] entry",
        )?
    } else {
        PathBuf::new()
    };
    validate_log_config(&log_config)?;
    if check_config {
        let policies = load_rate_policies(rate_limits_file.as_deref())?;
        if domains.is_empty() {
            let program = compile_file(&app)?;
            validate_route_rate_policies(&program, &policies)?;
            let exec = ExecutionLimits {
                max_instructions: config.max_instructions,
                max_allocated_bytes: config.max_runtime_alloc_bytes,
                max_external_io_bytes: config.max_runtime_alloc_bytes,
            };
            let profiles = load_resource_profiles(
                resource_profiles_file.as_deref(),
                &exec,
                config.max_connections,
            )?;
            for use_site in &program.resource_uses {
                if profiles.config(&use_site.profile).is_none() {
                    return Err(format!(
                        "{}:{} requests unknown resource profile `{}`",
                        use_site.source.file, use_site.source.line, use_site.profile
                    )
                    .into());
                }
            }
            for root in [static_assets.root.as_deref(), storage.data_root.as_deref()]
                .into_iter()
                .flatten()
            {
                let meta = fs::metadata(root)?;
                if !meta.is_dir() {
                    return Err(
                        format!("configured root `{}` is not a directory", root.display()).into(),
                    );
                }
            }
        } else {
            for domain in &domains {
                let program = compile_file(&domain.app)?;
                validate_route_rate_policies(&program, &policies)?;
                let exec = ExecutionLimits {
                    max_instructions: domain.config.max_instructions,
                    max_allocated_bytes: domain.config.max_runtime_alloc_bytes,
                    max_external_io_bytes: domain.config.max_runtime_alloc_bytes,
                };
                let profiles = load_resource_profiles(
                    domain.resource_profiles_file.as_deref(),
                    &exec,
                    domain.max_concurrent_requests,
                )?;
                for use_site in &program.resource_uses {
                    if profiles.config(&use_site.profile).is_none() {
                        return Err(format!(
                            "domain `{}` {}:{} requests unknown resource profile `{}`",
                            domain.host,
                            use_site.source.file,
                            use_site.source.line,
                            use_site.profile
                        )
                        .into());
                    }
                }
                for root in [
                    domain.static_assets.root.as_deref(),
                    domain.storage.data_root.as_deref(),
                ]
                .into_iter()
                .flatten()
                {
                    let meta = fs::metadata(root)?;
                    if !meta.is_dir() {
                        return Err(format!(
                            "domain `{}` root `{}` is not a directory",
                            domain.host,
                            root.display()
                        )
                        .into());
                    }
                }
            }
        }
        let _ = build_tls_acceptor(&tls, &domains)?;
        println!(
            "configuration OK: {}",
            config_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "CLI/defaults".into())
        );
        std::process::exit(0);
    }
    if print_effective {
        let effective_app = if domains.is_empty() {
            app.clone()
        } else {
            PathBuf::from("<multi-domain>")
        };
        print_effective_config(
            &effective_app,
            &config,
            &tls,
            &web,
            &storage,
            &static_assets,
            &lifecycle,
            &observability,
            &log_config,
            &cache_cli,
            &source_reload,
            &resource_limits,
            resource_profiles_file.as_deref(),
            rate_limits_file.as_deref(),
            &auth,
            db_configured,
        );
        crate::cli_effective::print_domains(&domains);
        std::process::exit(0);
    }
    Ok(StartupArgs {
        app,
        config,
        db_config,
        auth,
        tls,
        web,
        storage,
        resource_limits,
        resource_profiles_file,
        static_assets,
        lifecycle,
        rate_limits_file,
        allow_memory_rate_limit,
        observability,
        cache: cache_cli,
        native,
        log_config,
        domains,
        unix_socket,
        behind_proxy,
        source_reload,
    })
}
