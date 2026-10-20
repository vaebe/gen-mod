use super::tasks::TaskContext;
use serde_json::{json, Value};
pub fn expected_tool(title: &str) -> Option<&'static str> {
    match title {
        "gen-mod:read-project-context" => Some("read_project_context"),
        "gen-mod:read-project-memory-sources" => Some("read_project_memory_sources"),
        "gen-mod:update-project-memory" => Some("update_project_memory"),
        "gen-mod:check-delivery" => Some("prepare_delivery"),
        "gen-mod:read-feedback-sources" => Some("read_feedback_sources"),
        "gen-mod:propose-verification" => Some("propose_verification"),
        "gen-mod:search-knowledge" => Some("search_game_knowledge"),
        "gen-mod:save-long-term-memory" => Some("save_long_term_memory"),
        "gen-mod:read-game-inspection" => Some("read_game_inspection"),
        _ => None,
    }
}
pub fn execute(
    app: &tauri::AppHandle,
    task: &TaskContext,
    title: &str,
    request: &Value,
    input: &Value,
    secrets: &[String],
    cancelled: &std::sync::atomic::AtomicBool,
) -> Result<Value, String> {
    if cancelled.load(std::sync::atomic::Ordering::SeqCst) {
        return Err("工具已结束或取消".into());
    }
    let db = crate::storage::database::connect(app)?;
    match title {
        "gen-mod:read-project-context" => {
            let snapshot=crate::project_context::repository::snapshot_for_task_on(
                &db,
                &task.mod_id,
                &task.task_id,
            )?;
            Ok(json!({"live":crate::project_context::repository::read_on(&db,&task.mod_id)?,"snapshot":snapshot}))
        }
        "gen-mod:read-project-memory-sources" => crate::project_context::evidence::sources(app,task),
        "gen-mod:update-project-memory" => {
            if request["input"] != *input {
                return Err("项目记忆与工具参数不一致".into());
            }
            let result=crate::project_context::evidence::update_for_task(app,task,serde_json::from_value(input.clone()).map_err(|e|e.to_string())?,cancelled)?;
            Ok(json!({"projectMemoryUpdate":result}))
        }
        "gen-mod:check-delivery" => {
            if request["files"] != input["files"] {
                return Err("交付检查与工具参数不一致".into());
            }
            let files = input["files"]
                .as_array()
                .ok_or("缺少文件清单")?
                .iter()
                .map(|v| {
                    Ok((
                        v["source"].as_str().ok_or("缺少源路径")?.to_string(),
                        v["target"].as_str().ok_or("缺少目标路径")?.to_string(),
                    ))
                })
                .collect::<Result<Vec<_>, String>>()?;
            if files.is_empty() || files.len() > 1000 {
                return Err("交付文件数量无效".into());
            }
            let hashes = crate::artifacts::content_check::restricted_for_task(app, task)?;
            let report = crate::artifacts::content_check::check_files(
                std::path::Path::new(&task.source_directory),
                &files,
                secrets,
                &hashes,
            )?;
            Ok(
                json!({"checkReport":report,"blocked":report.findings.iter().any(|f|f.level=="block"),"scope":"候选文件字节与参考指纹；不展开嵌套压缩包"}),
            )
        }
        "gen-mod:read-feedback-sources" => {
            let mut sources =
                crate::verification::evidence::sources(app, &task.mod_id, &task.session_id)?;
            let versions = crate::artifacts::delivery_metadata::list_on(&db, &task.mod_id)?;
            sources["versions"]=json!(versions.iter().take(20).map(|v|json!({"versionId":v.version_id,"createdAt":v.created_at,"artifactDeleted":v.artifact_deleted,"cases":v.cases})).collect::<Vec<_>>());
            sources["olderVersionsAvailable"] = json!(versions.len() > 20);
            Ok(sources)
        }
        "gen-mod:propose-verification" => {
            if request["payload"] != *input {
                return Err("反馈与工具参数不一致".into());
            }
            let payload: crate::verification::models::FeedbackPayload =
                serde_json::from_value(input.clone()).map_err(|e| e.to_string())?;
            if payload
                .source_refs
                .iter()
                .any(|s| s.session_id != task.session_id)
            {
                return Err("反馈来源必须来自当前会话".into());
            }
            let evidence = crate::verification::evidence::validate(app, &task.mod_id, &payload)?;
            let mut writable=crate::storage::database::connect(app)?;
            let tx=writable.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
            crate::knowledge::repository::ensure_running_on(&tx,task,cancelled)?;
            let value=crate::verification::repository::propose_on(
                &tx,
                &task.mod_id,
                payload,
                evidence,
            )?;
            crate::knowledge::repository::ensure_running_on(&tx,task,cancelled)?;
            tx.commit().map_err(|e|e.to_string())?;
            serde_json::to_value(value).map_err(|e|e.to_string())
        }
        "gen-mod:search-knowledge" => {
            if request["terms"] != input["terms"] {
                return Err("检索参数不一致".into());
            }
            let terms: Vec<String> =
                serde_json::from_value(input["terms"].clone()).map_err(|e| e.to_string())?;
            if terms.len() > 10 || terms.iter().any(|s| s.len() > 200) {
                return Err("检索关键词过多或过长".into());
            }
            crate::knowledge::usage::search_for_task(app, task, &terms, cancelled)
        }
        "gen-mod:save-long-term-memory" => {
            if request["input"] != *input {
                return Err("记忆与工具参数不一致".into());
            }
            let saved = crate::knowledge::save_for_task(app, task,
                serde_json::from_value(input.clone()).map_err(|e| e.to_string())?, cancelled)?;
            Ok(json!({"memorySave":saved}))
        }
        "gen-mod:read-game-inspection" => {
            if request["reportId"] != input["reportId"] {
                return Err("调查参数不一致".into());
            }
            if let Some(id) = input["reportId"].as_str() {
                serde_json::to_value(
                    crate::game_inspection::repository::read_on(&db, &task.mod_id, id)?.0,
                )
                .map_err(|e| e.to_string())
            } else {
                serde_json::to_value(crate::game_inspection::repository::list_on(
                    &db,
                    &task.mod_id,
                )?)
                .map_err(|e| e.to_string())
            }
        }
        _ => Err("未知任务工具请求".into()),
    }
}
