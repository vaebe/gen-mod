use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallHint { pub description: String, pub example_path: String }

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallManifest { pub target_directory_hint: String, pub entries: Vec<InstallEntry> }
#[derive(Clone, Serialize, Deserialize)]
pub struct InstallEntry { pub source: String, pub target: String }
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Delivery { pub task_id: String, pub staging_id: String, pub command_ids: Vec<String>, pub manifest_path: String, pub zip_path: String, pub lease_id: String, pub source_revision: u64 }

use crate::storage::models::SkillSnapshot;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactFile {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactVersion {
    pub summary: String,
    pub session_id: String,
    pub version_id: String,
    pub task_id: String,
    pub created_at: String,
    pub skills: Vec<SkillSnapshot>,
    pub files: Vec<ArtifactFile>,
    pub zip_sha256: String,
}
