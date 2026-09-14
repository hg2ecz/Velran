use crate::server_errors::CliParseError;
use std::path::PathBuf;

pub(super) struct CliBootstrap {
    pub(super) config_path: Option<PathBuf>,
    pub(super) check_config: bool,
    pub(super) print_effective: bool,
}

pub(super) fn scan(raw_args: &[String]) -> Result<CliBootstrap, CliParseError> {
    let mut config_path: Option<PathBuf> = None;
    let mut check_config = false;
    let mut print_effective = false;
    let mut i = 0usize;
    while i < raw_args.len() {
        match raw_args[i].as_str() {
            "--config" => {
                if config_path.is_some() {
                    return Err("--config specified more than once".into());
                }
                i += 1;
                config_path = Some(PathBuf::from(
                    raw_args.get(i).ok_or("--config requires a path")?,
                ));
            }
            "--check-config" => check_config = true,
            "--print-effective-config" => print_effective = true,
            _ => {}
        }
        i += 1;
    }
    if check_config && print_effective {
        return Err("--check-config and --print-effective-config are mutually exclusive".into());
    }
    Ok(CliBootstrap {
        config_path,
        check_config,
        print_effective,
    })
}
