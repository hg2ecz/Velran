use executable_ir::Capability;
use executable_ir::shard_planner::VerifiedShard;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeTrust {
    FirstPartyVerified,
    ThirdPartyExtension,
    TenantIsolated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerRequirement {
    InProcessAllowed,
    WorkerRecommended,
    WorkerRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkerSandboxProfile {
    pub separate_uid: bool,
    pub read_only_root: bool,
    pub network_namespace: bool,
    pub syscall_filter: bool,
    pub resource_limits: bool,
}

impl WorkerSandboxProfile {
    pub const STRICT: Self = Self {
        separate_uid: true,
        read_only_root: true,
        network_namespace: true,
        syscall_filter: true,
        resource_limits: true,
    };
}

pub fn worker_requirement(trust: CodeTrust, shard: &VerifiedShard) -> WorkerRequirement {
    match trust {
        CodeTrust::ThirdPartyExtension | CodeTrust::TenantIsolated => {
            WorkerRequirement::WorkerRequired
        }
        CodeTrust::FirstPartyVerified if higher_risk_capabilities(shard) => {
            WorkerRequirement::WorkerRecommended
        }
        CodeTrust::FirstPartyVerified => WorkerRequirement::InProcessAllowed,
    }
}

fn higher_risk_capabilities(shard: &VerifiedShard) -> bool {
    shard.capabilities().iter().any(|capability| {
        matches!(
            capability,
            Capability::Network(_) | Capability::DbWrite | Capability::CriticalOperation(_)
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_profile_enables_all_expected_boundaries() {
        let profile = WorkerSandboxProfile::STRICT;
        assert!(profile.separate_uid && profile.read_only_root && profile.network_namespace);
        assert!(profile.syscall_filter && profile.resource_limits);
    }
}
