use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewEntry { pub source: String, pub target: String, pub overwrite: bool, pub before_hash: Option<String> }
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallPreview { pub preview_id: String, pub mod_id: String, pub version_id: String, pub target_directory: String, pub entries: Vec<PreviewEntry> }
