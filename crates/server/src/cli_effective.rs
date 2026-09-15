use crate::server_config_file::DomainCliConfig;

pub(super) fn print_domains(domains: &[DomainCliConfig]) {
    for domain in domains {
        println!("[[domains]]");
        println!("host = {:?}", domain.host);
        println!("aliases = {:?}", domain.aliases);
        println!("workdir = {:?}", domain.workdir.display().to_string());
        println!("app = {:?}", domain.app.display().to_string());
        println!("max_body_bytes = {}", domain.config.max_body_bytes);
        println!("request_timeout_ms = {}", domain.config.request_timeout_ms);
        println!("max_instructions = {}", domain.config.max_instructions);
        println!(
            "max_runtime_alloc_bytes = {}",
            domain.config.max_runtime_alloc_bytes
        );
        println!(
            "max_concurrent_requests = {}",
            domain.max_concurrent_requests
        );
        println!("max_queued_requests = {}", domain.max_queued_requests);
        println!("queue_timeout_ms = {}", domain.queue_timeout_ms);
        println!("reload_enabled = {}", domain.reload.enabled);
        println!("reload_mode = {}", domain.reload.mode.as_str());
        println!(
            "reload_poll_interval_ms = {}",
            domain.reload.poll_interval_ms
        );
        println!("reload_debounce_ms = {}", domain.reload.debounce_ms);
        println!(
            "reload_debug_compile_errors = {}",
            domain.reload.debug_compile_errors
        );
        println!(
            "tls_cert_file = {:?}",
            domain
                .tls
                .as_ref()
                .map(|v| v.cert_file.display().to_string())
        );
        println!(
            "tls_key_file = {:?}",
            domain
                .tls
                .as_ref()
                .map(|v| v.key_file.display().to_string())
        );
        println!(
            "resource_profiles_file = {:?}",
            domain
                .resource_profiles_file
                .as_ref()
                .map(|p| p.display().to_string())
        );
    }
}
