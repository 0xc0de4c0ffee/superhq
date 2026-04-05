//! Agent checkpoint utilities.
//!
//! On first use of an agent, the UI boots a temporary sandbox with networking,
//! runs the agent's install script, verifies it, then checkpoints the result.
//! Subsequent boots use the checkpoint for instant startup (~1s).
//!
//! The actual install flow is driven step-by-step from `TerminalPanel::open_agent_tab`
//! so that each phase can report progress to the UI.

use shuru_sdk::default_data_dir;

/// Returns the canonical checkpoint name for an agent.
pub fn agent_checkpoint_name(agent_slug: &str) -> String {
    format!("agent-{}-base", agent_slug)
}

/// Check if a checkpoint file exists on disk.
pub fn checkpoint_exists(checkpoint_name: &str) -> bool {
    let data_dir = default_data_dir();
    let path = format!("{}/checkpoints/{}.ext4", data_dir, checkpoint_name);
    std::path::Path::new(&path).exists()
}
