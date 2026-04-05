use crate::db::Workspace;

/// Build a SandboxConfig from workspace settings.
/// This will be fleshed out when we integrate shuru-sdk.
pub struct SandboxConfigBuilder;

impl SandboxConfigBuilder {
    pub fn from_workspace(_workspace: &Workspace) -> SandboxConfigBuilder {
        // TODO: Build shuru_sdk::SandboxConfig from workspace fields
        // - cpus, memory, disk from workspace
        // - mounts from mount_path
        // - network from allowed_hosts
        // - secrets from secrets_config
        // - from checkpoint if sandbox_checkpoint_name is set
        SandboxConfigBuilder
    }
}
