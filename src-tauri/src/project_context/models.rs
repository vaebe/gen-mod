use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextField {
    pub value: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    pub observed_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReferenceFile {
    pub path: String,
    pub sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReferenceSet {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_report_id: Option<String>,
    pub id: String,
    pub directory: String,
    pub files: Vec<ReferenceFile>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Prerequisite {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectContext {
    pub mod_id: String,
    pub revision: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game_version: Option<ContextField>,
    pub reference_sets: Vec<ReferenceSet>,
    pub prerequisites: Vec<Prerequisite>,
    pub confirmed_requirements: Vec<String>,
    #[serde(default)]
    pub field_sources: std::collections::BTreeMap<String,Vec<ProjectMemorySourceRef>>,
}
impl ProjectContext {
    pub fn empty(id: &str) -> Self {
        Self {
            mod_id: id.into(),
            revision: 0,
            game_version: None,
            reference_sets: vec![],
            prerequisites: vec![],
            confirmed_requirements: vec![],
            field_sources: Default::default(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextSnapshot {
    pub snapshot_id: String,
    pub mod_id: String,
    pub task_id: String,
    pub created_at: String,
    pub context: ProjectContext,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "field", rename_all = "camelCase", deny_unknown_fields)]
pub enum ContextChange {
    GameVersion {
        reason: String,
        value: String,
    },
    Prerequisites {
        reason: String,
        value: Vec<Prerequisite>,
    },
    ConfirmedRequirements {
        reason: String,
        value: Vec<String>,
    },
}
#[derive(Debug,Clone,Serialize,Deserialize,PartialEq)]
#[serde(tag="kind",rename_all="snake_case",rename_all_fields="camelCase",deny_unknown_fields)]
pub enum ProjectMemorySourceRef {
    UserMessage{session_id:String,entry_id:String,sha256:String},
    Command{task_id:String,command_id:String},
    Inspection{report_id:String},
}
#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct ProjectMemoryChange {
    pub field:String,pub reason:String,pub value:serde_json::Value,pub source_refs:Vec<ProjectMemorySourceRef>,
}
impl ProjectMemoryChange {
    pub fn change(&self)->Result<ContextChange,String>{
        serde_json::from_value(serde_json::json!({"field":self.field,"reason":self.reason,"value":self.value})).map_err(|e|e.to_string())
    }
}
#[derive(Debug,Clone,Serialize,Deserialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct ProjectMemoryUpdateInput {pub expected_revision:i64,pub changes:Vec<ProjectMemoryChange>}
#[derive(Debug,Serialize)]
#[serde(rename_all="camelCase")]
pub struct ProjectMemoryUpdateResult {pub outcome:&'static str,pub mod_id:String,pub revision:i64,pub changed_fields:Vec<String>}
