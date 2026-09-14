use crate::bootstrap_config::{parse_nonzero, parse_u64, parse_usize, read_secret_file};
use crate::cli_config_apply::LoadedCliConfig;
use crate::server_config_file::config_abs_path;
use crate::server_errors::CliParseError;
use crate::tls_support::validate_public_host;
use crate::validate_reserved_path;
use crate::web_security::valid_cors_origin;
use std::path::PathBuf;

pub(super) struct AppliedCli {
    pub(super) loaded: LoadedCliConfig,
    pub(super) force_disable_source_reload: bool,
}

pub(super) fn apply(
    raw_args: Vec<String>,
    loaded: LoadedCliConfig,
) -> Result<AppliedCli, CliParseError> {
    let LoadedCliConfig {
        mut app,
        mut config,
        mut db_url,
        mut resource_profiles_file,
        domain_entries,
        mut web,
        mut storage,
        mut static_assets,
        mut lifecycle,
        mut rate_limits_file,
        mut allow_memory_rate_limit,
        mut observability,
        mut log_config,
        mut cache_cli,
        mut native,
        mut source_reload,
        mut resource_limits,
        mut allow_insecure_db,
        mut auth,
        mut tls,
        mut unix_socket,
        mut behind_proxy,
    } = loaded;
    let mut args = raw_args.into_iter();
    let mut force_disable_source_reload = false;
    let mut db_cli_seen = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => {
                let _ = args.next().ok_or("--config requires a path")?;
            }
            "--check-config" | "--print-effective-config" => {}
            "--app" => app = args.next().map(PathBuf::from),
            "--listen" => {
                config.listen = args.next().ok_or("--listen requires an address")?.parse()?
            }
            "--unix-socket" => {
                unix_socket = Some(config_abs_path(
                    &args
                        .next()
                        .ok_or("--unix-socket requires an absolute path")?,
                    "--unix-socket",
                )?);
            }
            "--behind-proxy" => behind_proxy = true,
            "--tls-cert-file" => {
                tls.cert_file = Some(PathBuf::from(
                    args.next().ok_or("--tls-cert-file requires a path")?,
                ))
            }
            "--tls-key-file" => {
                tls.key_file = Some(PathBuf::from(
                    args.next().ok_or("--tls-key-file requires a path")?,
                ))
            }
            "--tls-handshake-timeout-ms" => {
                tls.handshake_timeout_ms = parse_u64(&mut args, "--tls-handshake-timeout-ms")?
            }
            "--http-redirect-listen" => {
                tls.http_redirect_listen = Some(
                    args.next()
                        .ok_or("--http-redirect-listen requires an address")?
                        .parse()?,
                )
            }
            "--public-host" => {
                tls.public_host = Some(validate_public_host(
                    &args.next().ok_or("--public-host requires a host")?,
                )?)
            }
            "--data-root" => {
                storage.data_root = Some(PathBuf::from(
                    args.next().ok_or("--data-root requires a path")?,
                ))
            }
            "--fs-mode" => {
                storage.fs_mode = args
                    .next()
                    .ok_or("--fs-mode requires rwc-style permissions")?
            }
            "--max-upload-bytes" => {
                storage.max_upload_bytes = parse_u64(&mut args, "--max-upload-bytes")?
            }
            "--max-image-pixels" => {
                storage.max_image_pixels = parse_u64(&mut args, "--max-image-pixels")?
            }
            "--static-root" => {
                static_assets.root = Some(PathBuf::from(
                    args.next().ok_or("--static-root requires a path")?,
                ))
            }
            "--static-url-prefix" => {
                static_assets.url_prefix =
                    args.next().ok_or("--static-url-prefix requires a value")?
            }
            "--max-static-asset-bytes" => {
                static_assets.max_asset_bytes = parse_u64(&mut args, "--max-static-asset-bytes")?
            }
            "--static-max-age-secs" => {
                static_assets.regular_max_age_secs = parse_u64(&mut args, "--static-max-age-secs")?
            }
            "--static-immutable-max-age-secs" => {
                static_assets.immutable_max_age_secs =
                    parse_u64(&mut args, "--static-immutable-max-age-secs")?
            }
            "--no-precompressed-static" => static_assets.precompressed = false,
            "--health-live-path" => {
                lifecycle.live_path = validate_reserved_path(
                    &args.next().ok_or("--health-live-path requires a path")?,
                )?
            }
            "--health-ready-path" => {
                lifecycle.ready_path = validate_reserved_path(
                    &args.next().ok_or("--health-ready-path requires a path")?,
                )?
            }
            "--health-dependency-timeout-ms" => {
                lifecycle.dependency_timeout_ms =
                    parse_u64(&mut args, "--health-dependency-timeout-ms")?
            }
            "--shutdown-grace-ms" => {
                lifecycle.shutdown_grace_ms = parse_u64(&mut args, "--shutdown-grace-ms")?
            }
            "--rate-limits-file" => {
                rate_limits_file = Some(PathBuf::from(
                    args.next().ok_or("--rate-limits-file requires a path")?,
                ))
            }
            "--allow-memory-rate-limit" => allow_memory_rate_limit = true,
            "--metrics-listen" => {
                observability.metrics_listen = Some(
                    args.next()
                        .ok_or("--metrics-listen requires an address")?
                        .parse()?,
                )
            }
            "--allow-public-metrics" => observability.allow_public_metrics = true,
            "--no-access-log" => observability.access_log = false,
            "--server-log-file" => {
                log_config.server_file = Some(PathBuf::from(
                    args.next().ok_or("--server-log-file requires a path")?,
                ))
            }
            "--access-log-file" => {
                log_config.access_file = Some(PathBuf::from(
                    args.next().ok_or("--access-log-file requires a path")?,
                ))
            }
            "--audit-log-file" => {
                log_config.audit_file = Some(PathBuf::from(
                    args.next().ok_or("--audit-log-file requires a path")?,
                ))
            }
            "--no-log-stderr" => log_config.stderr = false,
            "--cache-max-ttl-secs" => {
                cache_cli.max_ttl_secs = parse_u64(&mut args, "--cache-max-ttl-secs")?
            }
            "--cache-max-entries" => {
                cache_cli.max_entries = parse_nonzero(&mut args, "--cache-max-entries")?
            }
            "--cache-max-bytes" => {
                cache_cli.max_bytes = parse_nonzero(&mut args, "--cache-max-bytes")?
            }
            "--allow-memory-cache" => cache_cli.allow_memory = true,
            "--cache-singleflight-wait-timeout-ms" => {
                cache_cli.singleflight_wait_timeout_ms =
                    parse_u64(&mut args, "--cache-singleflight-wait-timeout-ms")?
            }
            "--no-source-reload" => {
                source_reload.enabled = false;
                force_disable_source_reload = true;
            }
            "--source-reload-poll-ms" => {
                source_reload.poll_interval_ms = parse_u64(&mut args, "--source-reload-poll-ms")?
            }
            "--source-reload-debounce-ms" => {
                source_reload.debounce_ms = parse_u64(&mut args, "--source-reload-debounce-ms")?
            }
            "--debug-compile-errors" => {
                source_reload.debug_compile_errors = true;
            }
            "--debug-rustc-repro" => {
                native.debug_rustc_repro = true;
            }
            "--max-process-memory-bytes" => {
                resource_limits.max_address_space_bytes =
                    Some(parse_u64(&mut args, "--max-process-memory-bytes")?)
            }
            "--cgroup-dir" => {
                resource_limits.cgroup_dir = Some(PathBuf::from(
                    args.next().ok_or("--cgroup-dir requires a path")?,
                ))
            }
            "--cgroup-memory-max-bytes" => {
                resource_limits.cgroup_memory_max_bytes =
                    Some(parse_u64(&mut args, "--cgroup-memory-max-bytes")?)
            }
            "--cgroup-memory-swap-max-bytes" => {
                resource_limits.cgroup_memory_swap_max_bytes =
                    Some(parse_u64(&mut args, "--cgroup-memory-swap-max-bytes")?)
            }
            "--cgroup-cpu-percent" => {
                resource_limits.cgroup_cpu_percent =
                    Some(parse_u64(&mut args, "--cgroup-cpu-percent")?.try_into()?)
            }
            "--cgroup-pids-max" => {
                resource_limits.cgroup_pids_max = Some(parse_u64(&mut args, "--cgroup-pids-max")?)
            }
            "--resource-profiles-file" => {
                resource_profiles_file = Some(PathBuf::from(
                    args.next()
                        .ok_or("--resource-profiles-file requires a path")?,
                ))
            }
            "--db-url" => {
                if db_cli_seen {
                    return Err("database URL specified more than once on CLI".into());
                }
                db_cli_seen = true;
                db_url = Some(args.next().ok_or("--db-url requires a database URL")?)
            }
            "--db-url-file" => {
                if db_cli_seen {
                    return Err("database URL specified more than once on CLI".into());
                }
                db_cli_seen = true;
                db_url = Some(read_secret_file(
                    &args.next().ok_or("--db-url-file requires a path")?,
                )?)
            }
            "--allow-insecure-db" => allow_insecure_db = true,
            "--redis-url-file" => {
                auth.redis_url = Some(read_secret_file(
                    &args.next().ok_or("--redis-url-file requires a path")?,
                )?)
            }
            "--allow-insecure-redis" => auth.allow_insecure_redis = true,
            "--ldap-url" => auth.ldap_url = args.next(),
            "--ldap-search-base" => auth.ldap_search_base = args.next(),
            "--ldap-username-attribute" => {
                auth.ldap_username_attribute = args
                    .next()
                    .ok_or("--ldap-username-attribute requires a value")?
            }
            "--ldap-service-bind-dn-file" => {
                auth.ldap_bind_dn = Some(read_secret_file(
                    &args
                        .next()
                        .ok_or("--ldap-service-bind-dn-file requires a path")?,
                )?)
            }
            "--ldap-service-bind-password-file" => {
                auth.ldap_bind_password =
                    Some(read_secret_file(&args.next().ok_or(
                        "--ldap-service-bind-password-file requires a path",
                    )?)?)
            }
            "--totp-secrets-file" => {
                auth.totp_secrets_file = Some(PathBuf::from(
                    args.next().ok_or("--totp-secrets-file requires a path")?,
                ))
            }
            "--auth-roles-file" => {
                auth.roles_file = Some(PathBuf::from(
                    args.next().ok_or("--auth-roles-file requires a path")?,
                ))
            }
            "--auth-memberships-file" => {
                auth.memberships_file = Some(PathBuf::from(
                    args.next()
                        .ok_or("--auth-memberships-file requires a path")?,
                ))
            }
            "--local-auth-db-url-file" => {
                auth.local_auth_db_url = Some(read_secret_file(
                    &args
                        .next()
                        .ok_or("--local-auth-db-url-file requires a path")?,
                )?)
            }
            "--require-totp" => auth.require_totp = true,
            "--login-max-attempts" => {
                auth.login_max_attempts =
                    parse_u64(&mut args, "--login-max-attempts")?.try_into()?
            }
            "--login-principal-max-attempts" => {
                auth.login_principal_max_attempts =
                    parse_u64(&mut args, "--login-principal-max-attempts")?.try_into()?
            }
            "--login-source-max-attempts" => {
                auth.login_source_max_attempts =
                    parse_u64(&mut args, "--login-source-max-attempts")?.try_into()?
            }
            "--mfa-max-attempts" => {
                auth.mfa_max_attempts = parse_u64(&mut args, "--mfa-max-attempts")?.try_into()?
            }
            "--login-window-secs" => {
                auth.login_window_secs = parse_u64(&mut args, "--login-window-secs")?
            }
            "--request-timeout-ms" => {
                config.request_timeout_ms = parse_u64(&mut args, "--request-timeout-ms")?
            }
            "--trusted-proxy-cidr" => {
                let raw = args.next().ok_or("--trusted-proxy-cidr requires CIDR")?;
                web.trusted_proxy_cidrs.push(raw.parse().map_err(|err| {
                    CliParseError::invalid(format!("invalid trusted proxy CIDR `{raw}`: {err}"))
                })?);
            }
            "--allow-missing-origin" => web.allow_missing_origin = true,
            "--cors-origin" => {
                let origin = args.next().ok_or("--cors-origin requires an origin")?;
                if !valid_cors_origin(&origin) {
                    return Err(format!("invalid CORS origin `{origin}`").into());
                }
                if web.cors_origins.contains(&origin) {
                    return Err(format!("duplicate CORS origin `{origin}`").into());
                }
                web.cors_origins.push(origin);
            }
            "--cors-allow-credentials" => web.cors_allow_credentials = true,
            "--max-header-bytes" => {
                config.max_header_bytes = parse_usize(&mut args, "--max-header-bytes")?
            }
            "--max-body-bytes" => {
                config.max_body_bytes = parse_usize(&mut args, "--max-body-bytes")?
            }
            "--max-connections" => {
                config.max_connections = parse_nonzero(&mut args, "--max-connections")?
            }
            "--max-requests-per-connection" => {
                config.max_requests_per_connection =
                    parse_nonzero(&mut args, "--max-requests-per-connection")?
            }
            "--read-timeout-ms" => {
                config.read_timeout_ms = parse_u64(&mut args, "--read-timeout-ms")?
            }
            "--write-timeout-ms" => {
                config.write_timeout_ms = parse_u64(&mut args, "--write-timeout-ms")?
            }
            "--max-header-count" => {
                config.max_header_count = parse_nonzero(&mut args, "--max-header-count")?
            }
            "--max-form-fields" => {
                config.max_form_fields = parse_nonzero(&mut args, "--max-form-fields")?
            }
            "--max-form-field-bytes" => {
                config.max_form_field_bytes = parse_nonzero(&mut args, "--max-form-field-bytes")?
            }
            "--max-json-depth" => {
                config.max_json_depth = parse_nonzero(&mut args, "--max-json-depth")?
            }
            "--max-json-string-bytes" => {
                config.max_json_string_bytes = parse_nonzero(&mut args, "--max-json-string-bytes")?
            }
            "--max-json-array-items" => {
                config.max_json_array_items = parse_nonzero(&mut args, "--max-json-array-items")?
            }
            "--max-json-object-fields" => {
                config.max_json_object_fields =
                    parse_nonzero(&mut args, "--max-json-object-fields")?
            }
            "--max-instructions" => {
                config.max_instructions = parse_u64(&mut args, "--max-instructions")?
            }
            "--max-runtime-alloc-bytes" => {
                config.max_runtime_alloc_bytes = parse_u64(&mut args, "--max-runtime-alloc-bytes")?
            }
            "--session-ttl-secs" => {
                config.session_ttl_secs = parse_u64(&mut args, "--session-ttl-secs")?
            }
            "--max-sessions" => config.max_sessions = parse_nonzero(&mut args, "--max-sessions")?,
            "--insecure-dev-cookies" => config.insecure_dev_cookies = true,
            "-h" | "--help" => crate::cli_help::print_and_exit(),
            other => return Err(format!("unknown argument `{other}`").into()),
        }
    }

    Ok(AppliedCli {
        loaded: LoadedCliConfig {
            app,
            config,
            db_url,
            resource_profiles_file,
            domain_entries,
            web,
            storage,
            static_assets,
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
        },
        force_disable_source_reload,
    })
}
