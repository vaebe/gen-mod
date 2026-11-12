use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceRef {
    pub kind: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KnowledgeInput {
    pub version_scope: String,
    pub game_version: Option<String>,
    pub route: String,
    pub title: String,
    pub content: String,
    pub evidence_kind: String,
    pub source_refs: Vec<SourceRef>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeNote {
    pub note_id: String,
    pub revision: i64,
    pub game_id: String,
    #[serde(flatten)]
    pub input: KnowledgeInput,
    pub status: String,
    pub allow_reuse: bool,
    #[serde(default)]
    pub origin_mod_id: Option<String>,
    #[serde(default)]
    pub origin_project_name: Option<String>,
    pub user_edited: bool,
    pub memory_key: String,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Serialize)]
pub struct KnowledgeReference {
    pub id: String,
    pub path: String,
    pub sha256: String,
    pub title: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryEntryInput {
    pub title: String,
    pub content: String,
    pub game_id: String,
    pub version_scope: String,
    pub game_version: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryListQuery {
    #[serde(default)]
    pub text: String,
    pub game_id: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryPage { pub notes: Vec<KnowledgeNote>, pub total: i64 }

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveLongTermMemoryInput {
    pub note_id: Option<String>,
    pub expected_revision: Option<i64>,
    pub topic_key: String,
    pub title: String,
    pub content: String,
    pub version_scope: String,
    pub game_version: Option<String>,
    pub evidence_kind: String,
    pub source_refs: Vec<SourceRef>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySaveResult {
    pub outcome: &'static str,
    pub note_id: String,
    pub revision: i64,
    pub title: String,
}
