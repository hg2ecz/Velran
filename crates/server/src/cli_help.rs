pub(super) fn print_and_exit() -> ! {
    println!(
        "Usage: velran-server [--config /usr/local/etc/velran/server.toml] --app <file.vrn> [overrides]"
    );
    println!(
        "Config: --config <file> [--check-config | --print-effective-config]; precedence defaults < config < CLI"
    );
    println!("HTTPS: --http-redirect-listen 0.0.0.0:80 --public-host example.com");
    println!(
        "Reverse proxy backend: --behind-proxy [--unix-socket /run/velran/velran.sock | --listen 127.0.0.1:8080]"
    );
    println!(
        "Web security: --request-timeout-ms 15000 --trusted-proxy-cidr <CIDR> [--allow-missing-origin] [--cors-origin https://frontend.example] [--cors-allow-credentials]"
    );
    println!(
        "Resource limits: --max-runtime-alloc-bytes <bytes> --max-process-memory-bytes <bytes> --resource-profiles-file <path> --cgroup-dir <path> --cgroup-memory-max-bytes <bytes> --cgroup-memory-swap-max-bytes <bytes> --cgroup-cpu-percent <n> --cgroup-pids-max <n>"
    );
    println!(
        "Storage/media: --data-root <path> [--fs-mode rwc] [--max-upload-bytes <bytes>] [--max-image-pixels <n>]"
    );
    println!(
        "Static assets: --static-root <path> [--static-url-prefix /assets/] [--max-static-asset-bytes <bytes>] [--static-max-age-secs <n>] [--static-immutable-max-age-secs <n>] [--no-precompressed-static]"
    );
    println!(
        "Lifecycle: [--health-live-path /health/live] [--health-ready-path /health/ready] [--health-dependency-timeout-ms 1000] [--shutdown-grace-ms 30000]"
    );
    println!(
        "Observability: [--metrics-listen 127.0.0.1:9090] [--allow-public-metrics] [--no-access-log]"
    );
    println!(
        "Logging/reload: [--server-log-file <path>] [--access-log-file <path>] [--audit-log-file <path>] [--no-log-stderr] [--no-source-reload] [--source-reload-poll-ms 1000] [--source-reload-debounce-ms 250] [--debug-compile-errors] [--debug-rustc-repro]; SIGHUP reloads domains/apps in behind-proxy mode; source changes are auto-reloaded transactionally"
    );
    println!("Rate limits: [--rate-limits-file <path>] [--allow-memory-rate-limit]");
    println!(
        "Public cache: [--cache-max-ttl-secs 3600] [--cache-max-entries 10000] [--cache-max-bytes 67108864] [--cache-singleflight-wait-timeout-ms 5000] [--allow-memory-cache]"
    );
    println!("Auth endpoints: GET/POST /__velran/auth/login, POST /__velran/auth/logout");
    println!("Auth backends: LDAP or --local-auth-db-url-file <path>");
    println!(
        "Production: use direct TLS, or loopback-only plain HTTP behind an explicitly trusted HTTPS reverse proxy with --public-host; --insecure-dev-cookies is local development only"
    );
    std::process::exit(0)
}
