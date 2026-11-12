use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewEntry { pub source: String, pub target: String, pub overwrite: bool, pub before_hash: Option<String> }
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallPreview { pub preview_id: String, pub mod_id: String, pub version_id: String, pub target_directory: String, pub entries: Vec<PreviewEntry> }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallationRecord {
    pub installation_id: String,
    pub version_id: String,
    pub target_directory: String,
    pub attempted_at: String,
    pub status: InstallationStatus,
    pub rollback: RollbackStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstallationStatus { Succeeded, Failed }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RollbackStatus { NotNeeded, NotAttempted }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledTarget {
    pub target_directory: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
    pub state: TargetState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TargetState { Known, Uncertain }
