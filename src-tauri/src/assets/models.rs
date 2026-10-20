use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Crop {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageRecipe {
    pub width: u32,
    pub height: u32,
    pub fit: String,
    pub crop: Option<Crop>,
    pub sampling: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAsset {
    pub asset_id: String,
    pub mod_id: String,
    pub original_name: String,
    pub relative_path: String,
    pub sha256: String,
    pub width: u32,
    pub height: u32,
    pub media_type: String,
    pub input_asset_id: Option<String>,
    pub recipe: Option<ImageRecipe>,
}
