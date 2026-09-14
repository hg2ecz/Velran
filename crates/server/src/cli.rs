use crate::cli_config_apply;
use crate::cli_finalize;
use crate::cli_overrides;
use crate::cli_scan;
use crate::server_errors::CliParseError;
use crate::startup_args::StartupArgs;
use std::env;

pub(super) fn parse_args() -> Result<StartupArgs, CliParseError> {
    let raw_args: Vec<String> = env::args().skip(1).collect();
    let bootstrap = cli_scan::scan(&raw_args)?;
    let loaded = cli_config_apply::load(bootstrap.config_path.as_deref())?;
    let applied = cli_overrides::apply(raw_args, loaded)?;
    cli_finalize::finalize(applied, bootstrap)
}
