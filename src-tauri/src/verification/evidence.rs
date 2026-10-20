use super::models::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::BufRead;
pub fn digest(value: &Value) -> Result<String, String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).map_err(|e| e.to_string())?)
    ))
}
pub(crate) fn user_messages(app: &tauri::AppHandle, id: &str, session_id: &str) -> Result<Vec<Value>, String> {
    let project = crate::storage::repository::read(app, id)?;
    let session = project
        .sessions
        .iter()
        .find(|s| s.session_id == session_id)
        .ok_or("证据会话不属于项目或已删除")?;
    if session.session_file.is_empty() {
        return Ok(vec![]);
    }
    let root = crate::storage::paths::project_dir(app, id)?.join("logs/sessions");
    let file = std::path::Path::new(&session.session_file);
    crate::storage::paths::reject_redirect(&root)?;
    crate::storage::paths::reject_redirect(file)?;
    if file.canonicalize().map_err(|e| e.to_string())?.parent()
        != Some(root.canonicalize().map_err(|e| e.to_string())?.as_path())
    {
        return Err("证据文件越界".into());
    }
    if std::fs::metadata(file).map_err(|e| e.to_string())?.len() > 128 * 1024 * 1024 {
        return Err("证据会话超过 128 MiB，请选择较小的会话".into());
    }
    let mut entries = vec![];
    for line in std::io::BufReader::new(std::io::Read::take(
        std::fs::File::open(file).map_err(|e| e.to_string())?,
        128 * 1024 * 1024 + 1,
    ))
    .lines()
    {
        let line = line.map_err(|e| e.to_string())?;
        let v: Value = serde_json::from_str(&line).map_err(|_| "会话证据文件损坏")?;
        if v["type"] == "message" && v["message"]["role"] == "user" {
            entries.push(v);
        }
    }
    Ok(entries)
}
fn summary(message: &Value) -> String {
    if let Some(text) = message["content"].as_str() {
        return text.chars().take(2000).collect();
    }
    message["content"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter(|v| v["type"] == "text")
                .filter_map(|v| v["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n")
                .chars()
                .take(2000)
                .collect()
        })
        .unwrap_or_default()
}
pub fn sources(app: &tauri::AppHandle, id: &str, session: &str) -> Result<Value, String> {
    let entries = user_messages(app, id, session)?;
    let mut output = vec![];
    for e in entries.iter().rev().take(50) {
        let attachments = e["message"]["content"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| v["type"] == "image")
                    .map(|(i, v)| {
                        Ok(json!({"index":i,"sha256":digest(v)?,"mimeType":v["mimeType"]}))
                    })
                    .collect::<Result<Vec<Value>, String>>()
            })
            .transpose()?
            .unwrap_or_default();
        output.push(json!({"entryId":e["id"],"sessionId":session,"text":summary(&e["message"]),"sha256":digest(&e["message"])?,"attachments":attachments}));
    }
    Ok(
        json!({"sources":output,"scope":"当前会话最近 50 条玩家消息；摘要最多 2000 字，不含图片原数据"}),
    )
}
pub fn validate(
    app: &tauri::AppHandle,
    id: &str,
    payload: &FeedbackPayload,
) -> Result<Vec<StoredEvidence>, String> {
    let mut saved = vec![];
    let mut unique = std::collections::BTreeSet::new();
    for source in &payload.source_refs {
        if !unique.insert((
            source.session_id.clone(),
            source.entry_id.clone(),
            source.attachment_index,
        )) {
            return Err("反馈证据重复".into());
        }
        let entries = user_messages(app, id, &source.session_id)?;
        let entry = entries
            .iter()
            .find(|v| v["id"] == source.entry_id)
            .ok_or("原玩家消息不存在，无法创建或修改草稿")?;
        let content = if let Some(index) = source.attachment_index {
            let v = entry["message"]["content"]
                .get(index)
                .ok_or("附件索引无效")?;
            if v["type"] != "image" {
                return Err("所选证据不是图片附件".into());
            }
            v
        } else {
            &entry["message"]
        };
        if digest(content)? != source.sha256 {
            return Err("证据指纹与原消息不一致".into());
        }
        saved.push(StoredEvidence {
            reference: source.clone(),
            summary: summary(&entry["message"]),
        });
    }
    Ok(saved)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn message_changes_invalidate_fingerprint() {
        let a = json!({"role":"user","content":[{"type":"text","text":"能加载"}]});
        let b = json!({"role":"user","content":[{"type":"text","text":"能加载但没效果"}]});
        assert_ne!(digest(&a).unwrap(), digest(&b).unwrap());
        assert_eq!(summary(&a), "能加载");
    }
}
