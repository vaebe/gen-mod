use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallManifest { pub target_directory_hint: String, pub entries: Vec<InstallEntry> }
#[derive(Clone, Serialize, Deserialize)]
pub struct InstallEntry { pub source: String, pub target: String }
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Delivery { pub task_id: String, pub staging_id: String, pub command_ids: Vec<String>, pub manifest_path: String, pub zip_path: String, pub lease_id: String, pub source_revision: u64 }
