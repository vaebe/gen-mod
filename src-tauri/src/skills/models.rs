use serde::{Deserialize, Serialize};

/// 分类与依赖是应用协议；pi 仅使用标准 SKILL.md 的 name/description。
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillDefinition {
    pub name: String,
    pub description: String,
    pub category: String,
    pub game_id: Option<String>,
    pub version: String,
    pub dependencies: Vec<String>,
    pub path: String,
    #[serde(default)]
    pub exclude_paths: Vec<String>,
    #[serde(default)]
    pub required_command_scripts: Vec<String>,
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub inspection_file: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillView {
    #[serde(flatten)]
    pub definition: SkillDefinition,
    pub source: String,
    pub enabled: bool,
    pub content_hash: String,
    pub scripts: Vec<String>,
    pub error: Option<String>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserRegistry {
    #[serde(default)]
    pub imported: Vec<SkillDefinition>,
    #[serde(default)]
    pub disabled: Vec<String>,
}
