use super::models::ContextSnapshot;
use crate::{pi::tasks::TaskContext,storage};
use serde::{Serialize,Deserialize};
use rusqlite::params;
use std::path::PathBuf;

#[derive(Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct MemoryBootstrap {
    pub mod_id:String,pub task_id:String,pub snapshot_id:String,pub revision:i64,pub summary:String,pub truncated:bool,pub history:Vec<History>,
}
#[derive(Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct History {pub task_id:String,pub status:String,#[serde(skip_serializing_if="Option::is_none")]pub version_id:Option<String>}
pub fn packet(task:&TaskContext,snapshot:&ContextSnapshot,history:Vec<History>,game:&str)->Result<MemoryBootstrap,String>{
    if snapshot.mod_id!=task.mod_id || snapshot.task_id!=task.task_id || task.context_snapshot_id.as_deref()!=Some(&snapshot.snapshot_id) {return Err("项目记忆快照身份不一致".into());}
    // 优先保留版本和玩家约束；参考程序集只带状态，不让文件清单挤占摘要。
    let context=&snapshot.context;
    let requirements=if context.confirmed_requirements.is_empty(){"尚未记录".into()}else{context.confirmed_requirements.join("\n")};
    let prerequisites=if context.prerequisites.is_empty(){"未知".into()}else{context.prerequisites.iter().map(|item|format!("{}：{}",item.name,item.version.as_deref().unwrap_or("版本未知"))).collect::<Vec<_>>().join("；")};
    let references=if context.reference_sets.is_empty(){"尚未选择".into()}else{context.reference_sets.iter().map(|set|format!("{}（{} 个参考文件，使用前核对指纹）",set.directory,set.files.len())).collect::<Vec<_>>().join("；")};
    let raw=format!("游戏：{game}\n游戏版本：{}\n已确认需求：\n{requirements}\n前置：{prerequisites}\n参考资料：{references}",context.game_version.as_ref().map(|field|field.value.as_str()).unwrap_or("未知"));
    let truncated=raw.chars().count()>6000;
    let summary=raw.chars().take(6000).collect::<String>();
    Ok(MemoryBootstrap{mod_id:task.mod_id.clone(),task_id:task.task_id.clone(),snapshot_id:snapshot.snapshot_id.clone(),revision:snapshot.context.revision,summary,truncated,history:history.into_iter().take(20).collect()})
}
pub fn write_bootstrap(app:&tauri::AppHandle,task:&TaskContext,snapshot:&ContextSnapshot)->Result<PathBuf,String>{
    let db=storage::database::connect(app)?;
    let mut statement=db.prepare("SELECT task_id,status,json_extract(payload,'$.registeredVersionId') FROM tasks WHERE mod_id=?1 AND task_id<>?2 ORDER BY created_at DESC,task_id DESC LIMIT 20").map_err(|e|e.to_string())?;
    let rows=statement.query_map(params![task.mod_id,task.task_id],|r|Ok(History{task_id:r.get(0)?,status:r.get(1)?,version_id:r.get(2)?})).map_err(|e|e.to_string())?;
    let history=rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    let project=storage::repository::read(app,&task.mod_id)?;
    let packet=packet(task,snapshot,history,&project.game_id)?;
    let root=storage::paths::project_dir(app,&task.mod_id)?;
    let directory=root.join("logs").join(&task.task_id);
    for ancestor in directory.ancestors(){storage::paths::reject_redirect(ancestor)?;}
    let path=directory.join("memory-context.json");
    if path.exists(){return Err("任务项目记忆文件已存在，不能覆盖".into());}
    storage::mods::write_json(&path,&packet)?;
    Ok(path)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn summary_is_bounded_and_legacy_context_remains_readable(){
        let raw=serde_json::json!({"taskId":"t","modId":"p","sessionId":"s","generation":1,"sourceDirectory":"source","stagingId":"stage","stagingDirectory":"stage","createdAt":"now"});
        let mut task:TaskContext=crate::storage::repository::decode(raw.to_string()).unwrap();
        assert!(task.memory_bootstrap_path.is_none());assert!(task.memory_bootstrap_sha256.is_none());
        task.context_snapshot_id=Some("snap".into());
        let mut context=super::super::models::ProjectContext::empty("p");context.confirmed_requirements=vec!["字".repeat(7000)];
        let snapshot=ContextSnapshot{snapshot_id:"snap".into(),mod_id:"p".into(),task_id:"t".into(),created_at:"now".into(),context};
        let result=packet(&task,&snapshot,vec![],"duckov").unwrap();assert_eq!(result.summary.chars().count(),6000);assert!(result.truncated);assert!(result.history.is_empty());
        assert!(result.summary.starts_with("游戏：duckov\n游戏版本：未知\n已确认需求："));
        task.mod_id="other".into();assert!(packet(&task,&snapshot,vec![],"duckov").is_err());
    }
}
