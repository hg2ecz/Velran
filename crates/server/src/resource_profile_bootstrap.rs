use crate::bootstrap_config::json_log_escape;
use crate::server_errors::ResourceProfileConfigError;
use observability::{audit_log, utc_timestamp};
use runtime::{ExecutionLimits, ResourceProfileConfig, ResourceProfiles};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub(super) fn load_resource_profiles(
    path: Option<&Path>,
    request: &ExecutionLimits,
    default_concurrency: usize,
) -> Result<ResourceProfiles, ResourceProfileConfigError> {
    let mut default = ResourceProfileConfig {
        max_instructions: request.max_instructions,
        max_allocated_bytes: request.max_allocated_bytes,
        max_external_io_bytes: request.max_external_io_bytes,
        max_concurrent: default_concurrency.max(1),
    };
    let mut named: HashMap<String, ResourceProfileConfig> = HashMap::new();
    let Some(path) = path else {
        return ResourceProfiles::new(default, named).map_err(ResourceProfileConfigError::from);
    };
    let text = fs::read_to_string(path).map_err(|source| ResourceProfileConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let mut section = String::new();
    let mut values: HashMap<String, HashMap<String, u64>> = HashMap::new();
    for (idx, raw) in text.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_string();
            if section != "default" && !section.starts_with("profile.") {
                return Err(ResourceProfileConfigError::Syntax {
                    path: path.to_path_buf(),
                    line: line_no,
                    message: format!("invalid resource profile section `{section}`"),
                });
            }
            continue;
        }
        if section.is_empty() {
            return Err(ResourceProfileConfigError::Syntax {
                path: path.to_path_buf(),
                line: line_no,
                message: "resource profile key outside a section".into(),
            });
        }
        let (key, val) =
            line.split_once('=')
                .ok_or_else(|| ResourceProfileConfigError::Syntax {
                    path: path.to_path_buf(),
                    line: line_no,
                    message: "expected key = value".into(),
                })?;
        let key = key.trim();
        if !matches!(
            key,
            "max_instructions" | "max_alloc_bytes" | "max_external_io_bytes" | "max_concurrent"
        ) {
            return Err(ResourceProfileConfigError::Syntax {
                path: path.to_path_buf(),
                line: line_no,
                message: format!("unknown resource profile key `{key}`"),
            });
        }
        let value =
            val.trim()
                .parse()
                .map_err(|source| ResourceProfileConfigError::InvalidNumber {
                    path: path.to_path_buf(),
                    line: line_no,
                    key: key.to_string(),
                    source,
                })?;
        values
            .entry(section.clone())
            .or_default()
            .insert(key.into(), value);
    }

    let build = |name: &str,
                 map: &HashMap<String, u64>|
     -> Result<ResourceProfileConfig, ResourceProfileConfigError> {
        let get = |field: &'static str| {
            map.get(field)
                .copied()
                .ok_or_else(|| ResourceProfileConfigError::MissingField {
                    profile: name.to_string(),
                    field,
                })
        };
        let max_concurrent_raw = get("max_concurrent")?;
        let config = ResourceProfileConfig {
            max_instructions: get("max_instructions")?,
            max_allocated_bytes: get("max_alloc_bytes")?,
            max_external_io_bytes: map
                .get("max_external_io_bytes")
                .copied()
                .unwrap_or(get("max_alloc_bytes")?)
                .min(request.max_external_io_bytes),
            max_concurrent: usize::try_from(max_concurrent_raw).map_err(|source| {
                ResourceProfileConfigError::ConcurrentOverflow {
                    profile: name.to_string(),
                    source,
                }
            })?,
        };
        if config.max_instructions > request.max_instructions
            || config.max_allocated_bytes > request.max_allocated_bytes
            || config.max_external_io_bytes > request.max_external_io_bytes
        {
            return Err(ResourceProfileConfigError::ExceedsRequestCeiling {
                profile: name.to_string(),
            });
        }
        Ok(config)
    };

    if let Some(values) = values
        .get("default")
        .or_else(|| values.get("profile.default"))
    {
        default = build("default", values)?;
    }
    for (section, values) in values {
        if section == "default" || section == "profile.default" {
            continue;
        }
        let name = section
            .strip_prefix("profile.")
            .expect("validated profile section")
            .to_string();
        named.insert(name.clone(), build(&name, &values)?);
    }
    ResourceProfiles::new(default, named).map_err(ResourceProfileConfigError::from)
}

pub(super) fn audit_resource_profiles(
    program: &language_core::Program,
    profiles: &ResourceProfiles,
) -> Result<(), ResourceProfileConfigError> {
    let default = profiles.default_config();
    for use_site in &program.resource_uses {
        let cfg = profiles.config(&use_site.profile).ok_or_else(|| {
            ResourceProfileConfigError::UnknownProfileUse {
                file: use_site.source.file.clone(),
                line: use_site.source.line,
                function: use_site.source.function.clone(),
                profile: use_site.profile.clone(),
            }
        })?;
        let elevated = cfg.max_instructions > default.max_instructions
            || cfg.max_allocated_bytes > default.max_allocated_bytes
            || cfg.max_external_io_bytes > default.max_external_io_bytes;
        audit_log(&format!(
            "{{\"timestamp\":\"{}\",\"event\":\"resource_profile_use\",\"file\":\"{}\",\"line\":{},\"function\":\"{}\",\"profile\":\"{}\",\"max_instructions\":{},\"max_alloc_bytes\":{},\"max_external_io_bytes\":{},\"max_concurrent\":{},\"elevated\":{}}}",
            utc_timestamp(),
            json_log_escape(&use_site.source.file),
            use_site.source.line,
            json_log_escape(&use_site.source.function),
            json_log_escape(&use_site.profile),
            cfg.max_instructions,
            cfg.max_allocated_bytes,
            cfg.max_external_io_bytes,
            cfg.max_concurrent,
            elevated
        ));
    }
    Ok(())
}
