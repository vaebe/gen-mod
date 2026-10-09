use serde::Serialize;
use std::{collections::HashMap, fs, path::PathBuf};
use tauri::Manager;

pub struct ModelConfig {
    pub base_url: String,
    pub model: String,
    pub key: String,
    pub env_path: PathBuf,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigurationStatus {
    base_url: String,
    model: String,
    key_configured: bool,
    env_path: String,
    runtime_version: String,
    node_version: String,
    runtime_prepared: bool,
}

/// 显式读取 env 文件，不改变宿主环境，不把凭据返回给前端。
pub fn read(app: &tauri::AppHandle) -> Result<ModelConfig, String> {
    let path = if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.env")
    } else {
        app.path().app_config_dir().map_err(|e| e.to_string())?.join(".env")
    };
    let mut fields = HashMap::new();
    if path.exists() {
        let iterator = dotenvy::from_path_iter(&path).map_err(|_| "无法读取 env 配置文件")?;
        for field in iterator {
            let (key, value) = field.map_err(|_| "env 配置格式错误，请检查引号及赋值格式")?;
            fields.insert(key, value);
        }
    }
    let base_url = fields.remove("STEPFUN_BASE_URL").unwrap_or_else(|| "https://api.stepfun.com/step_plan/v1".into()).trim_end_matches('/').to_string();
    let model = fields.remove("STEPFUN_MODEL").unwrap_or_else(|| "step-5-preview".into());
    let key = fields.remove("STEPFUN_API_KEY").unwrap_or_default().trim().to_string();
    if key.chars().any(|c| c.is_whitespace() || c.is_control()) { return Err("STEPFUN_API_KEY 不能包含空白或控制字符".into()); }
    let url = tauri::Url::parse(&base_url).map_err(|_| "STEPFUN_BASE_URL 不是有效 URL")?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() || url.query().is_some() || url.fragment().is_some() {
        return Err("STEPFUN_BASE_URL 应为不含凭据、查询或片段的 HTTP(S) 地址".into());
    }
    if model.trim().is_empty() || model.contains(['\n', '\r']) { return Err("STEPFUN_MODEL 不能为空或包含换行".into()); }
    Ok(ModelConfig { base_url, model, key, env_path: path })
}

pub fn prepare(app: &tauri::AppHandle, config: &ModelConfig) -> Result<PathBuf, String> {
    let directory = app.path().app_local_data_dir().map_err(|e| e.to_string())?.join("pi/config");
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let mut model = serde_json::json!({ "id": config.model });
    if config.model == "step-5-preview" {
        model["input"] = serde_json::json!(["text", "image"]);
        // StepFun 官方 Step 5 规格：1M 上下文、64k 输出、low/medium/high。
        model["reasoning"] = serde_json::json!(true);
        model["contextWindow"] = serde_json::json!(1_000_000);
        model["maxTokens"] = serde_json::json!(64_000);
        model["thinkingLevelMap"] = serde_json::json!({ "off": null, "minimal": null, "low": "low", "medium": "medium", "high": "high", "xhigh": null, "max": null });
    }
    let provider = serde_json::json!({ "providers": { "stepfun-plan": {
        "baseUrl": config.base_url, "api": "openai-responses", "apiKey": "$STEPFUN_API_KEY",
        "models": [model]
    }}});
    crate::storage::mods::write_json(&directory.join("models.json"), &provider)?;
    Ok(directory)
}

#[tauri::command]
pub fn get_configuration(app: tauri::AppHandle) -> Result<ConfigurationStatus, String> {
    let config = read(&app)?;
    let root = crate::pi::runtime::resources(&app)?;
    Ok(ConfigurationStatus {
        base_url: config.base_url, model: config.model,
        key_configured: !config.key.trim().is_empty(), env_path: config.env_path.to_string_lossy().into_owned(),
        runtime_version: "1.1.0".into(), node_version: "22.19.0".into(),
        runtime_prepared: root.join("node-v22.19.0-win-x64/node.exe").is_file() && root.join("pi-portable-v1.1.0/.deployment-complete").is_file(),
    })
}

#[tauri::command]
pub fn open_configuration_directory(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let config = read(&app)?;
    let directory = config.env_path.parent().ok_or("env 文件目录无效")?;
    fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    app.opener().open_path(directory.to_string_lossy().into_owned(), None::<&str>).map_err(|e| e.to_string())
}
