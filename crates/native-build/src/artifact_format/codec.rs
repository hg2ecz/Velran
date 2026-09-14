#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use super::{ArtifactManifest, ManifestDecodeError};

const FIELD_COUNT: usize = 14;

fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn decode(text: &str) -> Result<ArtifactManifest, ManifestDecodeError> {
    let mut fields = BTreeMap::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            return Err(ManifestDecodeError);
        };
        if fields.insert(key, value).is_some() {
            return Err(ManifestDecodeError);
        }
    }
    if fields.len() != FIELD_COUNT {
        return Err(ManifestDecodeError);
    }

    let get = |key: &str| fields.get(key).copied().ok_or(ManifestDecodeError);
    let artifact_sha256 = get("artifact_sha256")?;
    let generated_source_sha256 = get("generated_source_sha256")?;
    let interface_sha256 = get("interface_sha256")?;
    let implementation_sha256 = get("implementation_sha256")?;
    if !valid_hash(artifact_sha256)
        || !valid_hash(generated_source_sha256)
        || !valid_hash(interface_sha256)
        || !valid_hash(implementation_sha256)
    {
        return Err(ManifestDecodeError);
    }

    Ok(ArtifactManifest {
        manifest_version: get("manifest_version")?
            .parse()
            .map_err(|_| ManifestDecodeError)?,
        shard_id: get("shard_id")?.to_owned(),
        crate_name: get("crate_name")?.to_owned(),
        artifact_sha256: artifact_sha256.to_owned(),
        artifact_size: get("artifact_size")?
            .parse()
            .map_err(|_| ManifestDecodeError)?,
        generated_source_sha256: generated_source_sha256.to_owned(),
        interface_sha256: interface_sha256.to_owned(),
        implementation_sha256: implementation_sha256.to_owned(),
        runtime_abi_version: get("runtime_abi_version")?
            .parse()
            .map_err(|_| ManifestDecodeError)?,
        language_version: get("language_version")?.to_owned(),
        security_policy_version: get("security_policy_version")?.to_owned(),
        executable_ir_version: get("executable_ir_version")?
            .parse()
            .map_err(|_| ManifestDecodeError)?,
        codegen_version: get("codegen_version")?.to_owned(),
        contract_fingerprint: get("contract_fingerprint")?
            .parse()
            .map_err(|_| ManifestDecodeError)?,
    })
}

pub(super) fn encode(manifest: &ArtifactManifest) -> String {
    format!(
        "manifest_version={}\nshard_id={}\ncrate_name={}\nartifact_sha256={}\nartifact_size={}\ngenerated_source_sha256={}\ninterface_sha256={}\nimplementation_sha256={}\nruntime_abi_version={}\nlanguage_version={}\nsecurity_policy_version={}\nexecutable_ir_version={}\ncodegen_version={}\ncontract_fingerprint={}\n",
        manifest.manifest_version,
        manifest.shard_id,
        manifest.crate_name,
        manifest.artifact_sha256,
        manifest.artifact_size,
        manifest.generated_source_sha256,
        manifest.interface_sha256,
        manifest.implementation_sha256,
        manifest.runtime_abi_version,
        manifest.language_version,
        manifest.security_policy_version,
        manifest.executable_ir_version,
        manifest.codegen_version,
        manifest.contract_fingerprint,
    )
}
