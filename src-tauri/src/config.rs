use serde::{Deserialize, Serialize};
use std::{collections::{BTreeMap, HashSet}, fs, path::PathBuf, sync::Mutex};
use tauri::Manager;

#[derive(Default)]
pub struct ConfigurationState(pub Mutex<()>);

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDefinition {
    pub id: String,
    pub name: String,
    pub context_window: u32,
    pub max_tokens: u32,
    pub reasoning: bool,
    pub input: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub thinking_level_map: BTreeMap<String, Option<String>>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Provider {
    name: String,
    base_url: String,
    api: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    api_key: String,
    models: Vec<ModelDefinition>,
}

#[derive(Default, Deserialize, Serialize)]
struct ModelsFile {
    providers: BTreeMap<String, Provider>,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsFile {
    #[serde(default)]
    default_provider: String,
    #[serde(default)]
    default_model: String,
    #[serde(flatten)]
    other: BTreeMap<String, serde_json::Value>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfiguration {
    id: String,
    name: String,
    base_url: String,
    api: String,
    #[serde(default, skip_serializing)]
    api_key: Option<String>,
    #[serde(default)]
    key_configured: bool,
    models: Vec<ModelDefinition>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveConfiguration {
    providers: Vec<ProviderConfiguration>,
    default_provider: String,
    default_model: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigurationStatus {
    providers: Vec<ProviderConfiguration>,
    default_provider: String,
    default_model: String,
    model: String,
    key_configured: bool,
    configuration_directory: String,
    runtime_version: String,
    node_version: String,
    runtime_prepared: bool,
}

pub struct ModelConfig {
    pub provider: String,
    pub model: String,
    pub key: String,
    pub secrets: Vec<String>,
    definition: Provider,
    settings: SettingsFile,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSelection {
    pub provider_id: String,
    pub model_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionModelState {
    pub selection: ModelSelection,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<ModelDefinition>,
    pub error: String,
    pub thinking_level: String,
    pub thinking_levels: Vec<String>,
}

pub const THINKING_LEVELS: [&str; 7] = ["off", "minimal", "low", "medium", "high", "xhigh", "max"];

impl ModelDefinition {
    pub fn thinking_levels(&self) -> Vec<String> {
        if !self.reasoning { return vec!["off".into()]; }
        THINKING_LEVELS.into_iter().filter(|level| {
            match self.thinking_level_map.get(*level) {
                Some(None) => false,
                Some(Some(_)) => true,
                None => !matches!(*level, "xhigh" | "max"),
            }
        }).map(str::to_owned).collect()
    }
}

pub fn effective_thinking(levels: &[String], preferred: &str) -> String {
    if levels.iter().any(|level| level == preferred) { return preferred.into(); }
    // 不支持已保存偏好时，采用最接近 medium 的档位；同距优先较低档位。
    levels.iter().min_by_key(|level| THINKING_LEVELS.iter().position(|value| *value == level.as_str()).unwrap_or(0).abs_diff(3))
        .cloned().unwrap_or_else(|| "off".into())
}

pub fn session_model_state(app: &tauri::AppHandle, selection: Option<ModelSelection>, preferred: &str) -> SessionModelState {
    let mut result = SessionModelState { selection: selection.clone().unwrap_or(ModelSelection { provider_id: String::new(), model_id: String::new() }), definition: None, error: String::new(), thinking_level: "off".into(), thinking_levels: vec![] };
    let inspect = (|| -> Result<(), String> {
        let state = app.state::<ConfigurationState>();
        let _guard = state.0.lock().map_err(|_| "配置锁异常")?;
        let (models, settings) = load(app)?;
        if selection.is_none() { result.selection = ModelSelection { provider_id: settings.default_provider, model_id: settings.default_model }; }
        let mut provider = models.providers.get(&result.selection.provider_id).cloned().ok_or("聊天所选模型服务不存在，请重新选择模型")?;
        restore_thinking_map(&mut provider);
        let model = provider.models.iter().find(|model| model.id == result.selection.model_id).cloned().ok_or("聊天所选模型不存在，请重新选择模型")?;
        result.thinking_levels = model.thinking_levels();
        result.thinking_level = effective_thinking(&result.thinking_levels, preferred);
        result.definition = Some(model);
        if provider.api_key.is_empty() { return Err("所选模型服务未配置 API Key，请在设置页填写或选择其他模型".into()); }
        if result.thinking_levels.is_empty() { return Err("所选模型没有可用的思考强度，请检查模型配置".into()); }
        Ok(())
    })();
    if let Err(error) = inspect { result.error = error; }
    result
}

pub(crate) fn directory(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app.path().app_config_dir().map_err(|e| e.to_string())?.join("pi"))
}

fn load(app: &tauri::AppHandle) -> Result<(ModelsFile, SettingsFile), String> {
    let root = directory(app)?;
    fn read<T: serde::de::DeserializeOwned + Default>(path: PathBuf) -> Result<T, String> {
        match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|_| format!("{} 格式错误", path.file_name().unwrap_or_default().to_string_lossy())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
            Err(_) => Err("无法读取模型配置文件".into()),
        }
    }
    Ok((read(root.join("models.json"))?, read(root.join("settings.json"))?))
}

fn restore_thinking_map(provider: &mut Provider) {
    // 恢复原 Step Plan 协议约束，不将同名模型或参数套用到其他服务。
    if provider.api != "openai-responses" || provider.base_url.trim_end_matches('/') != "https://api.stepfun.com/step_plan/v1" { return; }
    for model in &mut provider.models {
        if model.id == "step-5-preview" && model.reasoning && model.thinking_level_map.is_empty() {
            model.thinking_level_map = ["off", "minimal", "low", "medium", "high", "xhigh", "max"].into_iter()
                .map(|level| (level.into(), if matches!(level, "low" | "medium" | "high") { Some(level.into()) } else { None })).collect();
        }
    }
}

fn selected<'a>(models: &'a ModelsFile, settings: &SettingsFile) -> Result<&'a Provider, String> {
    let provider = models.providers.get(&settings.default_provider).ok_or("请在设置页添加 provider 并选择默认模型")?;
    if !provider.models.iter().any(|model| model.id == settings.default_model) {
        return Err("默认模型不存在，请在设置页重新选择".into());
    }
    if provider.api_key.is_empty() { return Err("请在设置页填写默认 provider 的 API Key；本地无鉴权服务可填任意占位值".into()); }
    Ok(provider)
}

// 只读取应用配置目录，不读取项目 env，也不向前端返回已保存的 Key。
pub fn read_for(app: &tauri::AppHandle, selection: &ModelSelection) -> Result<ModelConfig, String> {
    let state = app.state::<ConfigurationState>();
    let _guard = state.0.lock().map_err(|_| "配置锁异常")?;
    let (models, mut settings) = load(app)?;
    settings.default_provider = selection.provider_id.clone();
    settings.default_model = selection.model_id.clone();
    let provider = selected(&models, &settings)?;
    let mut secrets: Vec<String> = models.providers.values().map(|item| item.api_key.clone()).filter(|key| !key.is_empty()).collect();
    // JSON 日志中的引号和反斜杠会转义，原文与转义文本都需遮蔽。
    let escaped: Vec<String> = secrets.iter().filter_map(|key| serde_json::to_string(key).ok())
        .map(|key| key[1..key.len() - 1].to_string()).collect();
    secrets.extend(escaped);
    secrets.sort_by_key(|key| std::cmp::Reverse(key.len()));
    secrets.dedup();
    Ok(ModelConfig { provider: settings.default_provider.clone(), model: settings.default_model.clone(), key: provider.api_key.clone(), secrets, definition: provider.clone(), settings })
}

pub fn read_session(app: &tauri::AppHandle, mod_id: &str, session_id: &str) -> Result<ModelConfig, String> {
    let (selection, preferred) = crate::storage::repository::model_preferences(app, mod_id, session_id)?;
    let state = session_model_state(app, selection, &preferred);
    if !state.error.is_empty() { return Err(state.error); }
    read_for(app, &state.selection)
}

impl ModelConfig {
    pub fn model_definition(&self) -> Result<ModelDefinition, String> {
        let mut provider = self.definition.clone();
        restore_thinking_map(&mut provider);
        provider.models.into_iter().find(|model| model.id == self.model).ok_or_else(|| "所选模型不存在".into())
    }
    pub fn prepare(&self) -> Result<tempfile::TempDir, String> {
        // 每个进程独占配置快照；pi 的设置写入不会触及用户配置或其他进程。
        let root = tempfile::Builder::new().prefix("gen-mod-pi-").tempdir().map_err(|_| "无法创建运行时配置目录")?;
        let mut provider = self.definition.clone();
        restore_thinking_map(&mut provider);
        provider.api_key = "$GEN_MOD_PROVIDER_API_KEY".into();
        provider.models.retain(|model| model.id == self.model);
        let models = ModelsFile { providers: BTreeMap::from([(self.provider.clone(), provider)]) };
        crate::storage::mods::write_json(&root.path().join("models.json"), &models).map_err(|_| "无法写入运行时模型配置")?;
        crate::storage::mods::write_json(&root.path().join("settings.json"), &self.settings).map_err(|_| "无法写入运行时默认模型")?;
        Ok(root)
    }
}

fn status(app: &tauri::AppHandle, models: ModelsFile, settings: SettingsFile) -> Result<ConfigurationStatus, String> {
    let key_configured = selected(&models, &settings).is_ok();
    let root = crate::pi::runtime::resources(app)?;
    Ok(ConfigurationStatus {
        providers: models.providers.into_iter().map(|(id, provider)| ProviderConfiguration {
            id, name: provider.name, base_url: provider.base_url, api: provider.api,
            key_configured: !provider.api_key.is_empty(), api_key: None, models: provider.models,
        }).collect(),
        model: settings.default_model.clone(), default_provider: settings.default_provider,
        default_model: settings.default_model, key_configured,
        configuration_directory: directory(app)?.to_string_lossy().into_owned(),
        runtime_version: "1.5.0".into(), node_version: "26.12.1".into(),
        runtime_prepared: root.join("node-v26.12.1-win-x64/node.exe").is_file() && root.join("pi-portable-v1.5.0/.deployment-complete").is_file(),
    })
}

#[tauri::command]
pub fn get_configuration(app: tauri::AppHandle) -> Result<ConfigurationStatus, String> {
    let state = app.state::<ConfigurationState>();
    let _guard = state.0.lock().map_err(|_| "配置锁异常")?;
    let (models, settings) = load(&app)?;
    status(&app, models, settings)
}

#[tauri::command]
pub fn save_configuration(app: tauri::AppHandle, configuration: SaveConfiguration) -> Result<ConfigurationStatus, String> {
    let state = app.state::<ConfigurationState>();
    let _guard = state.0.lock().map_err(|_| "配置锁异常")?;
    let (previous, previous_settings) = load(&app)?;
    let mut models = ModelsFile::default();
    for item in configuration.providers {
        let id = item.id.trim().to_string();
        if id.is_empty() || !id.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.')) {
            return Err("Provider ID 只能包含字母、数字、点、横线和下划线".into());
        }
        if models.providers.contains_key(&id) { return Err("Provider ID 不能重复".into()); }
        let name = item.name.trim().to_string();
        if name.is_empty() { return Err("Provider 名称不能为空".into()); }
        let base_url = item.base_url.trim().trim_end_matches('/').to_string();
        let url = tauri::Url::parse(&base_url).map_err(|_| "Base URL 不是有效 URL")?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() || url.query().is_some() || url.fragment().is_some() {
            return Err("Base URL 应为不含凭据、查询或片段的 HTTP(S) 地址".into());
        }
        if !matches!(item.api.as_str(), "openai-completions" | "openai-responses" | "anthropic-messages" | "google-generative-ai") { return Err("不支持的 API 协议".into()); }
        // 留空保留同 ID 的 Key；表单只接受实际密钥，不执行命令或环境变量表达式。
        let entered_key = item.api_key.unwrap_or_default().trim().to_string();
        let key = if entered_key.is_empty() { previous.providers.get(&id).map(|p| p.api_key.clone()).unwrap_or_default() } else { entered_key };
        if key.chars().any(|c| c.is_whitespace() || c.is_control()) || key.starts_with('!') || key.contains('$') {
            return Err("API Key 应为实际密钥，不能包含空白、控制字符、命令或环境变量表达式".into());
        }
        let mut ids = HashSet::new();
        let mut definitions = item.models;
        for model in &mut definitions {
            model.id = model.id.trim().to_string();
            model.name = model.name.trim().to_string();
            if model.id.is_empty() || model.id.chars().any(|c| c.is_whitespace() || c.is_control()) || model.name.is_empty() { return Err("模型 ID 不能为空或包含空白，显示名称不能为空".into()); }
            if !ids.insert(model.id.clone()) { return Err("同一 provider 下的模型 ID 不能重复".into()); }
            if model.context_window == 0 || model.max_tokens == 0 || model.max_tokens > model.context_window { return Err("模型上下文和最大输出必须为正整数，最大输出不能超过上下文长度".into()); }
            if model.input != ["text"] && model.input != ["text", "image"] { return Err("模型输入能力配置无效".into()); }
            if model.thinking_level_map.iter().any(|(level, value)| !matches!(level.as_str(), "off" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max") || value.as_ref().is_some_and(|value| value.trim().is_empty() || value.chars().any(|c| c.is_whitespace() || c.is_control()))) {
                return Err("模型思考强度映射无效".into());
            }
        }
        models.providers.insert(id, Provider { name, base_url, api: item.api, api_key: key, models: definitions });
    }
    let settings = SettingsFile {
        default_provider: configuration.default_provider, default_model: configuration.default_model,
        other: previous_settings.other.clone(),
    };
    if !settings.default_provider.is_empty() || !settings.default_model.is_empty() {
        let provider = models.providers.get(&settings.default_provider).ok_or("默认 provider 不存在")?;
        if !provider.models.iter().any(|model| model.id == settings.default_model) { return Err("默认模型不存在".into()); }
    }
    let result = status(&app, ModelsFile { providers: models.providers.clone() }, SettingsFile {
        default_provider: settings.default_provider.clone(), default_model: settings.default_model.clone(), other: settings.other.clone(),
    })?;
    let root = directory(&app)?;
    fs::create_dir_all(&root).map_err(|_| "无法创建模型配置目录")?;
    crate::storage::mods::write_json(&root.join("models.json"), &models).map_err(|_| "无法保存模型配置")?;
    if crate::storage::mods::write_json(&root.join("settings.json"), &settings).is_err() {
        crate::storage::mods::write_json(&root.join("models.json"), &previous).map_err(|_| "默认模型保存失败，恢复 provider 配置也失败，请重新打开设置页")?;
        return Err("默认模型保存失败，provider 配置已恢复".into());
    }
    Ok(result)
}

#[tauri::command]
pub fn open_configuration_directory(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let root = directory(&app)?;
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    app.opener().open_path(root.to_string_lossy().into_owned(), None::<&str>).map_err(|e| e.to_string())
}

pub fn cloud_api_url(app: &tauri::AppHandle) -> Result<String, String> {
 let state=app.state::<ConfigurationState>();let _guard=state.0.lock().map_err(|_|"配置锁异常")?;let (_,settings)=load(app)?;
 Ok(settings.other.get("cloudApiUrl").and_then(|v|v.as_str()).unwrap_or("http://127.0.0.1:8787").into())
}
pub fn save_cloud_api_url(app: &tauri::AppHandle, url:&str) -> Result<(), String> {
 let state=app.state::<ConfigurationState>();let _guard=state.0.lock().map_err(|_|"配置锁异常")?;let (_,mut settings)=load(app)?;settings.other.insert("cloudApiUrl".into(),serde_json::json!(url));let root=directory(app)?;fs::create_dir_all(&root).map_err(|e|e.to_string())?;crate::storage::mods::write_json(&root.join("settings.json"),&settings)
}
