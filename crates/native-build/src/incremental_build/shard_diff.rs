#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use executable_ir::shard_planner::VerifiedShard;

use crate::build_cache::{CacheKey, NativeBuildCache};
use crate::rustc_backend::RustcConfig;

pub(super) fn shard_key_map(
    shards: &[VerifiedShard],
    cache: &NativeBuildCache,
    rustc: &RustcConfig,
) -> BTreeMap<String, String> {
    shards
        .iter()
        .map(|shard| {
            let key: CacheKey = cache.key_for(shard, &rustc.cache_identity());
            (shard.id().as_str().to_owned(), key.as_str().to_owned())
        })
        .collect()
}

pub(super) fn dirty_shards<'a>(
    shards: &'a [VerifiedShard],
    previous: Option<&BTreeMap<String, String>>,
    current: &BTreeMap<String, String>,
) -> Vec<&'a VerifiedShard> {
    shards
        .iter()
        .filter(|shard| {
            let id = shard.id().as_str();
            previous.and_then(|keys| keys.get(id)) != current.get(id)
        })
        .collect()
}

pub(super) fn shard_interface_map(shards: &[VerifiedShard]) -> BTreeMap<String, String> {
    shards
        .iter()
        .map(|shard| {
            (
                shard.id().as_str().to_owned(),
                shard.interface_sha256().to_owned(),
            )
        })
        .collect()
}

pub(super) fn shard_implementation_map(shards: &[VerifiedShard]) -> BTreeMap<String, String> {
    shards
        .iter()
        .map(|shard| {
            (
                shard.id().as_str().to_owned(),
                shard.implementation_sha256().to_owned(),
            )
        })
        .collect()
}

pub(super) fn changed_fingerprints(
    previous: Option<&BTreeMap<String, String>>,
    current: &BTreeMap<String, String>,
) -> Vec<String> {
    current
        .iter()
        .filter_map(|(id, hash)| {
            (previous.and_then(|values| values.get(id)) != Some(hash)).then(|| id.clone())
        })
        .collect()
}

pub(super) fn removed_shards(
    previous: Option<&BTreeMap<String, String>>,
    current: &BTreeMap<String, String>,
) -> Vec<String> {
    previous
        .into_iter()
        .flat_map(|keys| keys.keys())
        .filter(|id| !current.contains_key(*id))
        .cloned()
        .collect()
}
