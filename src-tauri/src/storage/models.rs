use crate::{artifacts::models::ArtifactVersion, install::models::{InstallationRecord, InstalledTarget}};
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
pub struct SessionReference {
    pub session_id: String,
    pub session_file: String,
    pub skills: Vec<SkillSnapshot>,
    pub read_only: bool,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DevelopmentStatus { Draft, Generated, Running, Failed, Ready }

/// 稳定项目元数据；历史安装成功记录不代表目标当前仍安装该版本。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModProject {
    pub revision: i64,
    pub archived: bool,
    pub mod_id: String,
    pub name: String,
    pub game_id: String,
    pub game_skill_name: String,
    pub description: String,
    pub draft_requirement: String,
    pub updated_at: String,
    pub development_status: DevelopmentStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version_id: Option<String>,
    /// 最近选择且通过预览校验的目录；新项目尚未选择时为空。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_deployment_directory: Option<String>,
    pub versions: Vec<ArtifactVersion>,
    pub installations: Vec<InstallationRecord>,
    pub installed_targets: Vec<InstalledTarget>,
    pub sessions: Vec<SessionReference>,
}
