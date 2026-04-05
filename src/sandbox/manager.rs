use crate::db::{CreateWorkspaceParams, Database};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

/// Manages sandbox lifecycles. Holds references to running sandboxes.
/// Will hold shuru_sdk::AsyncSandbox instances once integrated.
pub struct SandboxManager {
    db: Arc<Database>,
    /// Map of workspace_id -> running sandbox handle (placeholder for now)
    _running: HashMap<i64, ()>,
}

impl SandboxManager {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            _running: HashMap::new(),
        }
    }

    /// Boot a new sandbox for a workspace.
    pub async fn boot(&mut self, workspace_id: i64) -> Result<()> {
        // TODO: Integrate shuru-sdk
        // 1. Load workspace from DB
        // 2. Build SandboxConfig from workspace fields
        // 3. AsyncSandbox::boot(config)
        // 4. Store sandbox handle in self._running
        // 5. Update DB status='running'

        self.db
            .update_workspace_status(workspace_id, crate::db::WorkspaceStatus::Running)?;

        Ok(())
    }

    /// Stop a sandbox and clean up.
    pub async fn stop(&mut self, workspace_id: i64) -> Result<()> {
        // TODO: Integrate shuru-sdk
        // 1. sandbox.stop()
        // 2. Remove from self._running
        // 3. Update DB status

        self._running.remove(&workspace_id);
        self.db
            .update_workspace_status(workspace_id, crate::db::WorkspaceStatus::Stopped)?;

        Ok(())
    }

    /// Clone a workspace: checkpoint the source, boot a new sandbox from it.
    pub async fn clone_workspace(
        &mut self,
        source_workspace_id: i64,
        new_name: String,
    ) -> Result<i64> {
        // TODO: Integrate shuru-sdk
        // 1. source_sandbox.checkpoint("clone-{timestamp}")
        // 2. Create new workspace in DB (cloned_from_id = source)
        // 3. Boot new sandbox from checkpoint

        let workspaces = self.db.list_workspaces()?;
        let source = workspaces
            .iter()
            .find(|w| w.id == source_workspace_id)
            .ok_or_else(|| anyhow::anyhow!("workspace not found"))?;

        let new_id = self.db.create_workspace(CreateWorkspaceParams {
            name: new_name,
            mount_path: source.mount_path.clone(),
            mount_read_only: true,
            is_git_repo: source.is_git_repo,
            branch_name: source.branch_name.clone(),
            base_branch: source.base_branch.clone(),
            initial_prompt: None,
            sandbox_cpus: source.sandbox_cpus,
            sandbox_memory_mb: source.sandbox_memory_mb,
            sandbox_disk_mb: source.sandbox_disk_mb,
            allowed_hosts: source.allowed_hosts.clone(),
            secrets_config: source.secrets_config.clone(),
            cloned_from_id: Some(source_workspace_id),
        })?;

        self.boot(new_id).await?;
        Ok(new_id)
    }
}
