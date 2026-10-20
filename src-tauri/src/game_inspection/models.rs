use serde::{Deserialize, Serialize};
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InspectionRule {
    pub schema_version: u32,
    pub profiles: Vec<Profile>,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Profile {
    pub kind: String,
    pub paths: Vec<RulePath>,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RulePath {
    pub relative: String,
    pub fact: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub relative_path: String,
    pub fact: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanError {
    pub relative_path: String,
    pub code: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectionReport {
    pub report_id: String,
    pub mod_id: String,
    pub game_id: String,
    pub kind: String,
    pub inspected_at: String,
    pub findings: Vec<Finding>,
    pub unknowns: Vec<String>,
    pub truncated: bool,
    pub errors: Vec<ScanError>,
}
