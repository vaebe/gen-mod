use super::models::*;
use crate::{pi::tasks::{TaskContext,TaskRecord},storage::{database,repository::{decode,task}}};
use rusqlite::{Connection,params};
use serde_json::{Value,json};

fn message_text(message:&Value)->String {
    message["content"].as_str().map(str::to_owned).unwrap_or_else(||message["content"].as_array().map(|items|items.iter().filter_map(|v|v["text"].as_str()).collect::<Vec<_>>().join("\n")).unwrap_or_default())
}
pub fn validate_on(db:&Connection,task:&TaskContext,input:&ProjectMemoryUpdateInput,entries:&[Value])->Result<(),String>{
    if input.changes.is_empty() || input.changes.len()>3{return Err("项目记忆变更数量无效".into());}
    for change in &input.changes {
        let converted=change.change()?;
        if change.source_refs.is_empty() || change.source_refs.len()>20{return Err("项目记忆需要 1–20 条真实来源".into());}
        let mut user=false;let mut seen=std::collections::BTreeSet::new();
        for source in &change.source_refs {
            if !seen.insert(serde_json::to_string(source).map_err(|e|e.to_string())?){return Err("项目记忆来源重复".into());}
            match source {
                ProjectMemorySourceRef::UserMessage{session_id,entry_id,sha256}=>{
                    crate::knowledge::evidence::validate_message(entries,task,&crate::knowledge::models::SourceRef{kind:"user_message".into(),id:entry_id.clone(),session_id:Some(session_id.clone()),sha256:Some(sha256.clone()),..Default::default()})?;
                    user=true;
                    if let ContextChange::GameVersion{value,..}=&converted {
                        let entry=entries.iter().find(|e|e["id"]==*entry_id).ok_or("原消息不存在")?;
                        if !message_text(&entry["message"]).contains(value){return Err("用户消息未明确包含所填版本，有歧义时先询问".into());}
                    }
                },
                ProjectMemorySourceRef::Command{task_id,command_id}=>{
                    let raw:String=db.query_row("SELECT payload FROM tasks WHERE mod_id=?1 AND task_id=?2",params![task.mod_id,task_id],|r|r.get(0)).map_err(|_|"命令来源不属于当前项目")?;
                    let record:TaskRecord=decode(raw)?;
                    if record.context.mod_id!=task.mod_id || record.context.task_id!=*task_id{return Err("命令任务身份不一致".into());}
                    let receipt=record.commands.iter().find(|r|r.command_id==*command_id && r.task_id==*task_id).ok_or("没有对应的真实命令回执")?;
                    match &converted {
                        ContextChange::Prerequisites{value,..}=>{
                            for item in value {
                                if let Some(version)=&item.version {
                                    if receipt.status!="succeeded" || receipt.exit_code!=Some(0) || version.is_empty() || !receipt.output_tail.contains(version)
                                        || !(receipt.program.to_lowercase().contains(&item.name.to_lowercase()) || receipt.args.iter().any(|arg|arg.to_lowercase().contains(&item.name.to_lowercase()))) {
                                        return Err("前置版本缺少对应成功探测回执，请保留未知或核对来源".into());
                                    }
                                }
                            }
                        },
                        _=>return Err("命令回执不能替代明确的用户需求或游戏版本来源".into()),
                    }
                },
                ProjectMemorySourceRef::Inspection{report_id}=>{
                    let (report,_)=crate::game_inspection::repository::read_on(db,&task.mod_id,report_id)?;
                    if report.mod_id!=task.mod_id || report.report_id!=*report_id{return Err("调查报告身份不一致".into());}
                    // 当前规则只确认文件标记和程序集，不提供游戏版本值。
                    if matches!(converted,ContextChange::GameVersion{..}){return Err("当前调查没有游戏版本事实，不能从 DLL 推断版本".into());}
                    if !matches!(converted,ContextChange::Prerequisites{..}){return Err("调查结果不能替代玩家需求".into());}
                    if let ContextChange::Prerequisites{value,..}=&converted {if value.iter().any(|p|p.version.is_some() || !report.findings.iter().any(|f|f.fact.eq_ignore_ascii_case(&p.name))){return Err("前置条件未出现在调查事实中".into());}}
                },
            }
        }
        if matches!(converted,ContextChange::ConfirmedRequirements{..}) && !user {return Err("已确认需求必须关联当前会话的用户表达".into());}
    }
    Ok(())
}
pub fn sources(app:&tauri::AppHandle,context:&TaskContext)->Result<Value,String>{
    let mut result=crate::verification::evidence::sources(app,&context.mod_id,&context.session_id)?;
    let record=task(app,&context.mod_id,&context.task_id)?;
    result["commands"]=json!(record.commands.iter().rev().take(20).map(|r|json!({"taskId":r.task_id,"commandId":r.command_id,"status":r.status,"exitCode":r.exit_code,"program":r.program,"args":r.args,"outputTail":r.output_tail.chars().take(2000).collect::<String>()})).collect::<Vec<_>>());
    let memory=super::repository::read_on(&database::connect(app)?,&context.mod_id)?;
    let ids:std::collections::BTreeSet<_>=memory.reference_sets.iter().filter_map(|set|set.source_report_id.clone()).collect();
    result["inspectionReportIds"]=json!(ids);
    Ok(result)
}
pub fn update_for_task(app:&tauri::AppHandle,task:&TaskContext,input:ProjectMemoryUpdateInput,cancelled:&std::sync::atomic::AtomicBool)->Result<ProjectMemoryUpdateResult,String>{
    let entries=if input.changes.iter().any(|c|c.source_refs.iter().any(|s|matches!(s,ProjectMemorySourceRef::UserMessage{..}))){crate::verification::evidence::user_messages(app,&task.mod_id,&task.session_id)?}else{vec![]};
    let mut db=database::connect(app)?;
    validate_on(&db,task,&input,&entries)?;
    let result=super::repository::update_automatic_on(&mut db,task,&input,cancelled)?;
    if result.outcome=="saved" {
        use tauri::Emitter;
        app.emit("project-memory-changed",json!({"modId":result.mod_id,"revision":result.revision})).map_err(|e|format!("项目记忆已保存，但更新通知发送失败：{e}"))?;
    }
    Ok(result)
}
