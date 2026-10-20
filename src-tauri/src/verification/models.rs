use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum VerificationResult {
    Passed,
    Partial,
    Failed,
    Unknown,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceRef {
    pub session_id: String,
    pub entry_id: String,
    pub attachment_index: Option<usize>,
    pub sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaseResult {
    pub case_id: String,
    pub result: VerificationResult,
    pub observed: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FeedbackPayload {
    pub version_id: String,
    pub game_version: Option<String>,
    pub observations: String,
    pub result: VerificationResult,
    pub case_results: Vec<CaseResult>,
    pub source_refs: Vec<EvidenceRef>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredEvidence {
    #[serde(rename = "ref")]
    pub reference: EvidenceRef,
    pub summary: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackDraft {
    pub draft_id: String,
    pub mod_id: String,
    pub revision: i64,
    pub status: String,
    pub payload: FeedbackPayload,
    pub evidence: Vec<StoredEvidence>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationRecord {
    pub record_id: String,
    pub mod_id: String,
    pub version_id: String,
    pub source_draft_id: String,
    pub confirmed_at: String,
    pub payload: FeedbackPayload,
    pub evidence: Vec<StoredEvidence>,
    pub supersedes: Option<String>,
}
#[derive(Serialize)]
pub struct VerificationList {
    pub drafts: Vec<FeedbackDraft>,
    pub records: Vec<VerificationRecord>,
}
