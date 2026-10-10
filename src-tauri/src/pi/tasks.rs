use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskContext {
    pub task_id: String,
    pub mod_id: String,
    pub session_id: String,
    pub generation: u64,
    pub source_directory: String,
    pub staging_id: String,
    pub staging_directory: String,
    pub created_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandReceipt {
    #[serde(default)]
    pub source_revision: Option<u64>,
    pub task_id: String,
    pub command_id: String,
    pub tool_call_id: String,
    pub exit_code: Option<i32>,
    pub status: String,
    pub log_path: String,
    #[serde(default)]
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRecord {
    pub context: TaskContext,
    pub status: String,
    #[serde(default)]
    pub finished_at: Option<String>,
    pub commands: Vec<CommandReceipt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registered_version_id: Option<String>,
}
