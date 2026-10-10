use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSnapshot {
    pub name: String,
    pub version: String,
    pub content_hash: String,
}

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
    pub version_id: String,
    pub task_id: String,
    pub created_at: String,
    pub skills: Vec<SkillSnapshot>,
    pub files: Vec<ArtifactFile>,
    pub zip_sha256: String,
}

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
    pub backup_directory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstallationStatus { Succeeded, Failed }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RollbackStatus { NotNeeded, NotAttempted, Succeeded, Failed }

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionReference {
    pub session_id: String,
    pub session_file: String,
    pub skills: Vec<SkillSnapshot>,
    pub read_only: bool,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DevelopmentStatus { Draft, Generated, Running, Failed, Ready }

/// 稳定项目元数据；历史安装成功记录不代表目标当前仍安装该版本。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModProject {
    #[serde(default, skip)]
    pub revision: i64,
    pub mod_id: String,
    pub name: String,
    pub game_id: String,
    pub game_skill_name: String,
    pub draft_requirement: String,
    pub updated_at: String,
    pub development_status: DevelopmentStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version_id: Option<String>,
    /// 最近选择且通过预览校验的目录；旧元数据没有此字段。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_deployment_directory: Option<String>,
    pub versions: Vec<ArtifactVersion>,
    pub installations: Vec<InstallationRecord>,
    pub installed_targets: Vec<InstalledTarget>,
    pub sessions: Vec<SessionReference>,
}
