use crate::storage::{
    database,
    models::{DevelopmentStatus, ModProject, SessionReference, SkillSnapshot},
    repository::{decode, encode, read_on},
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
pub enum SessionChange {
    History {
        session_id: String,
        file: String,
    },
    RefreshSkills {
        current: Option<Vec<SkillSnapshot>>,
    },
    Runtime {
        status: DevelopmentStatus,
        title: Option<(String, String)>,
        touch: bool,
    },
    Rename {
        session_id: String,
        name: String,
    },
    Delete {
        session_id: String,
    },
    Create {
        session: SessionReference,
        model: Option<crate::config::ModelSelection>,
    },
}
/// 会话写入与项目 revision 在同一事务提交，不重写版本、安装或其他会话。
pub fn edit_on(
    db: &mut Connection,
    id: &str,
    expected_revision: i64,
    change: SessionChange,
) -> Result<ModProject, String> {
    let tx = db
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    apply_on(&tx, id, expected_revision, change)?;
    let project = read_on(&tx, id)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(project)
}
fn apply_on(
    tx: &Connection,
    id: &str,
    expected_revision: i64,
    change: SessionChange,
) -> Result<(), String> {
    crate::project_lifecycle::repository::ensure_writable_on(tx, id)?;
    match change {
        SessionChange::History { session_id, file } => {
            require_session(tx.execute("UPDATE sessions SET payload=json_set(payload,'$.sessionFile',?3) WHERE mod_id=?1 AND session_id=?2 AND deleted=0",params![id,session_id,file]).map_err(|e|e.to_string())?)?;
        }
        SessionChange::RefreshSkills { current } => {
            let mut sessions = read_on(&tx, id)?.sessions;
            crate::skills::registry::refresh_snapshots(&mut sessions, current.as_ref());
            for session in sessions {
                tx.execute("UPDATE sessions SET payload=json_set(payload,'$.readOnly',json(?3)) WHERE mod_id=?1 AND session_id=?2 AND deleted=0",params![id,session.session_id,encode(&session.read_only)?]).map_err(|e|e.to_string())?;
            }
        }
        SessionChange::Runtime {
            status,
            title,
            touch: should_touch,
        } => {
            // 同项目其他会话仍活动时，单个会话终态不能把项目标成结束。
            let active:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM tasks WHERE mod_id=?1 AND status IN ('preparing','running','stopping'))",[id],|r|r.get(0)).map_err(|e|e.to_string())?;
            let status = if active {
                DevelopmentStatus::Running
            } else {
                status
            };
            tx.execute("UPDATE projects SET payload=json_set(payload,'$.developmentStatus',json(?2)) WHERE mod_id=?1",params![id,encode(&status)?]).map_err(|e|e.to_string())?;
            if let Some((session_id, name)) = title {
                tx.execute("UPDATE sessions SET payload=json_set(payload,'$.name',?3) WHERE mod_id=?1 AND session_id=?2 AND deleted=0 AND json_extract(payload,'$.name')=''",params![id,session_id,name]).map_err(|e|e.to_string())?;
            }
            if should_touch {
                touch(&tx, id)?;
            }
        }
        SessionChange::Rename { session_id, name } => {
            require_session(tx.execute("UPDATE sessions SET payload=json_set(payload,'$.name',?3) WHERE mod_id=?1 AND session_id=?2 AND deleted=0",params![id,session_id,name]).map_err(|e|e.to_string())?)?;
            touch(&tx, id)?;
        }
        SessionChange::Delete { session_id } => {
            require_session(
                tx.execute(
                    "UPDATE sessions SET deleted=1 WHERE mod_id=?1 AND session_id=?2 AND deleted=0",
                    params![id, session_id],
                )
                .map_err(|e| e.to_string())?,
            )?;
            touch(&tx, id)?;
        }
        SessionChange::Create { session, model } => {
            let has_sessions: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sessions WHERE mod_id=?1 AND deleted=0)",
                    [id],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            let draft: String = if has_sessions {
                String::new()
            } else {
                tx.query_row("SELECT json_extract(payload,'$.draftRequirement') FROM projects WHERE mod_id=?1",[id],|r|r.get(0)).map_err(|e|e.to_string())?
            };
            tx.execute("INSERT INTO sessions(mod_id,session_id,payload,draft,provider_id,model_id) VALUES(?1,?2,?3,?4,?5,?6)",params![id,session.session_id,encode(&session)?,draft,model.as_ref().map(|m|&m.provider_id),model.as_ref().map(|m|&m.model_id)]).map_err(|e|e.to_string())?;
            if !has_sessions {
                tx.execute("UPDATE projects SET payload=json_set(payload,'$.draftRequirement','') WHERE mod_id=?1",[id]).map_err(|e|e.to_string())?;
            }
        }
    }
    if tx
        .execute(
            "UPDATE projects SET revision=revision+1 WHERE mod_id=?1 AND revision=?2",
            params![id, expected_revision],
        )
        .map_err(|e| e.to_string())?
        != 1
    {
        return Err("项目元数据已更新，请重新读取后操作".into());
    }
    Ok(())
}
fn require_session(count: usize) -> Result<(), String> {
    if count != 1 {
        return Err("会话不存在或已删除".into());
    }
    Ok(())
}
fn touch(db: &Connection, id: &str) -> Result<(), String> {
    db.execute(
        "UPDATE projects SET payload=json_set(payload,'$.updatedAt',?2) WHERE mod_id=?1",
        params![id, crate::storage::mods::now()?],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
pub fn edit(app: &tauri::AppHandle, id: &str, change: SessionChange) -> Result<ModProject, String> {
    let mut db = database::connect(app)?;
    let tx = db
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    let revision = read_on(&tx, id)?.revision;
    apply_on(&tx, id, revision, change)?;
    let project = read_on(&tx, id)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(project)
}
pub fn refresh_skills(app: &tauri::AppHandle, id: &str) -> Result<ModProject, String> {
    let project = crate::storage::repository::read(app, id)?;
    let current = crate::skills::registry::resolve_for_game(app, &project.game_id)
        .map(|v| crate::skills::registry::snapshots(&v))
        .ok();
    edit(app, id, SessionChange::RefreshSkills { current })
}
pub fn save_preferences(
    app: &tauri::AppHandle,
    mod_id: &str,
    session_id: &str,
    draft: Option<&str>,
    thinking: Option<&str>,
) -> Result<(), String> {
    let db = database::connect(app)?;
    crate::project_lifecycle::repository::ensure_writable_on(&db, mod_id)?;
    let count = db.execute("UPDATE sessions SET draft=COALESCE(?3,draft),thinking_level=COALESCE(?4,thinking_level) WHERE mod_id=?1 AND session_id=?2 AND deleted=0",params![mod_id,session_id,draft,thinking]).map_err(|e| e.to_string())?;
    if count != 1 {
        return Err("会话不存在或已删除".into());
    }
    Ok(())
}

pub fn save_model(
    app: &tauri::AppHandle,
    mod_id: &str,
    session_id: &str,
    selection: Option<&crate::config::ModelSelection>,
) -> Result<(), String> {
    let mut db = database::connect(app)?;
    let tx = db.transaction().map_err(|e| e.to_string())?;
    crate::project_lifecycle::repository::ensure_writable_on(&tx, mod_id)?;
    // 更换模型后旧的上下文窗口统计不再适用，不能在重开聊天时继续显示。
    tx.execute("DELETE FROM session_stats WHERE mod_id=?1 AND session_id=?2 AND EXISTS(SELECT 1 FROM sessions WHERE mod_id=?1 AND session_id=?2 AND (provider_id IS NOT ?3 OR model_id IS NOT ?4))", params![mod_id,session_id,selection.map(|value| &value.provider_id),selection.map(|value| &value.model_id)]).map_err(|e| e.to_string())?;
    let count = tx.execute("UPDATE sessions SET provider_id=?3,model_id=?4 WHERE mod_id=?1 AND session_id=?2 AND deleted=0", params![mod_id,session_id,selection.map(|value| &value.provider_id),selection.map(|value| &value.model_id)]).map_err(|e| e.to_string())?;
    if count != 1 {
        return Err("会话不存在或已删除".into());
    }
    tx.commit().map_err(|e| e.to_string())
}

pub fn save_stats(
    app: &tauri::AppHandle,
    mod_id: &str,
    session_id: &str,
    generation: u64,
    usage: &serde_json::Value,
) -> Result<(), String> {
    let db = database::connect(app)?;
    let sampled = crate::storage::mods::now()?;
    let mut usage = usage.clone();
    if usage["tokens"].is_null() {
        let old: Option<String> = db
            .query_row(
                "SELECT payload FROM session_stats WHERE mod_id=?1 AND session_id=?2",
                params![mod_id, session_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if let Some(old) = old {
            let previous: serde_json::Value = decode(old)?;
            if usage["contextWindow"].is_null() {
                usage["contextWindow"] = previous["contextWindow"].clone();
            }
            usage["previous"] = if previous["tokens"].is_null() {
                previous["previous"].clone()
            } else {
                previous
            };
        }
    }
    usage["generation"] = generation.into();
    usage["sampledAt"] = sampled.clone().into();
    usage["estimated"] = true.into();
    db.execute("INSERT INTO session_stats VALUES(?1,?2,?3,?4) ON CONFLICT(mod_id,session_id) DO UPDATE SET sampled_at=excluded.sampled_at,payload=excluded.payload",params![mod_id,session_id,sampled,encode(&usage)?]).map_err(|e|e.to_string())?;
    Ok(())
}

/// 条件清理只比较原会话草稿；不匹配保持成功且保留新输入。
pub fn save_draft_on(
    db: &mut Connection,
    id: &str,
    session_id: &str,
    draft: &str,
    expected: Option<&str>,
) -> Result<(), String> {
    let tx = db
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    crate::project_lifecycle::repository::ensure_writable_on(&tx, id)?;
    let count=tx.execute("UPDATE sessions SET draft=?3 WHERE mod_id=?1 AND session_id=?2 AND deleted=0 AND (?4 IS NULL OR draft=?4)",params![id,session_id,draft,expected]).map_err(|e|e.to_string())?;
    if expected.is_none() {
        require_session(count)?;
    }
    tx.commit().map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    fn fixture() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(include_str!("../storage/schema.sql"))
            .unwrap();
        let payload = serde_json::json!({"modId":"p","name":"project","gameId":"duckov","gameSkillName":"duckov-mod","revision":0,"archived":false,"description":"description","draftRequirement":"  initial\n草稿  ","updatedAt":"before","developmentStatus":"draft","versions":[],"installations":[],"installedTargets":[]});
        db.execute(
            "INSERT INTO projects(mod_id,payload) VALUES('p',?1)",
            [payload.to_string()],
        )
        .unwrap();
        db
    }
    fn session(id: &str) -> crate::storage::models::SessionReference {
        serde_json::from_value(serde_json::json!({"sessionId":id,"sessionFile":"","skills":[],"readOnly":false,"name":""})).unwrap()
    }
    #[test]
    fn first_session_claims_draft_once_and_failure_rolls_back() {
        let mut db = fixture();
        db.execute_batch("CREATE TRIGGER refuse_clear BEFORE UPDATE ON projects BEGIN SELECT RAISE(ABORT,'fail'); END;").unwrap();
        assert!(edit_on(
            &mut db,
            "p",
            0,
            SessionChange::Create {
                session: session("s"),
                model: None
            }
        )
        .is_err());
        assert!(crate::storage::repository::read_on(&db, "p")
            .unwrap()
            .sessions
            .is_empty());
        assert_eq!(
            crate::storage::repository::read_on(&db, "p")
                .unwrap()
                .draft_requirement,
            "  initial\n草稿  "
        );
        db.execute_batch("DROP TRIGGER refuse_clear;").unwrap();
        let first = edit_on(
            &mut db,
            "p",
            0,
            SessionChange::Create {
                session: session("s"),
                model: None,
            },
        )
        .unwrap();
        assert_eq!(first.draft_requirement, "");
        let draft: String = db
            .query_row("SELECT draft FROM sessions WHERE session_id='s'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(draft, "  initial\n草稿  ");
        edit_on(
            &mut db,
            "p",
            first.revision,
            SessionChange::Create {
                session: session("second"),
                model: None,
            },
        )
        .unwrap();
        let drafts: Vec<String> = db
            .prepare("SELECT draft FROM sessions ORDER BY rowid")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(drafts, vec!["  initial\n草稿  ", ""]);
    }
    #[test]
    fn local_session_changes_preserve_other_data_and_reject_stale_or_archived_writes() {
        let mut db = fixture();
        edit_on(
            &mut db,
            "p",
            0,
            SessionChange::Create {
                session: session("s"),
                model: None,
            },
        )
        .unwrap();
        let before = read_on(&db, "p").unwrap();
        assert!(edit_on(
            &mut db,
            "p",
            0,
            SessionChange::Rename {
                session_id: "s".into(),
                name: "stale".into()
            }
        )
        .is_err());
        assert_eq!(read_on(&db, "p").unwrap().sessions[0].name, "");
        let named = edit_on(
            &mut db,
            "p",
            before.revision,
            SessionChange::Rename {
                session_id: "s".into(),
                name: "title".into(),
            },
        )
        .unwrap();
        assert_eq!(named.name, before.name);
        assert_eq!(named.description, before.description);
        assert_eq!(named.sessions[0].name, "title");
        assert_eq!(
            db.query_row::<String, _, _>(
                "SELECT draft FROM sessions WHERE session_id='s'",
                [],
                |r| r.get(0)
            )
            .unwrap(),
            "  initial\n草稿  "
        );
        db.execute("UPDATE projects SET archived=1", []).unwrap();
        assert!(edit_on(
            &mut db,
            "p",
            named.revision,
            SessionChange::Delete {
                session_id: "s".into()
            }
        )
        .is_err());
        assert_eq!(read_on(&db, "p").unwrap().sessions.len(), 1);
        db.execute("UPDATE projects SET archived=0", []).unwrap();
        let removed = edit_on(
            &mut db,
            "p",
            named.revision,
            SessionChange::Delete {
                session_id: "s".into(),
            },
        )
        .unwrap();
        assert!(removed.sessions.is_empty());
    }

    #[test]
    fn runtime_updates_preserve_drafts_titles_and_unrelated_project_fields() {
        let mut db = fixture();
        let first = edit_on(
            &mut db,
            "p",
            0,
            SessionChange::Create {
                session: session("s"),
                model: None,
            },
        )
        .unwrap();
        let titled = edit_on(
            &mut db,
            "p",
            first.revision,
            SessionChange::Rename {
                session_id: "s".into(),
                name: "user title".into(),
            },
        )
        .unwrap();
        let done = edit_on(
            &mut db,
            "p",
            titled.revision,
            SessionChange::Runtime {
                status: crate::storage::models::DevelopmentStatus::Generated,
                title: Some(("s".into(), "generated title".into())),
                touch: true,
            },
        )
        .unwrap();
        assert_eq!(done.sessions[0].name, "user title");
        assert_eq!(done.description, "description");
        let history = edit_on(
            &mut db,
            "p",
            done.revision,
            SessionChange::History {
                session_id: "s".into(),
                file: "history".into(),
            },
        )
        .unwrap();
        assert_eq!(history.sessions[0].session_file, "history");
        let refreshed = edit_on(
            &mut db,
            "p",
            history.revision,
            SessionChange::RefreshSkills { current: None },
        )
        .unwrap();
        assert!(refreshed.sessions[0].read_only);
        assert_eq!(
            db.query_row::<String, _, _>(
                "SELECT draft FROM sessions WHERE session_id='s'",
                [],
                |r| r.get(0)
            )
            .unwrap(),
            "  initial\n草稿  "
        );
    }

    #[test]
    fn draft_cleanup_matches_original_input_and_preserves_new_edits() {
        let mut db = fixture();
        edit_on(
            &mut db,
            "p",
            0,
            SessionChange::Create {
                session: session("s"),
                model: None,
            },
        )
        .unwrap();
        save_draft_on(&mut db, "p", "s", "edited", None).unwrap();
        save_draft_on(&mut db, "p", "s", "", Some("old sent input")).unwrap();
        assert_eq!(
            db.query_row::<String, _, _>(
                "SELECT draft FROM sessions WHERE session_id='s'",
                [],
                |r| r.get(0)
            )
            .unwrap(),
            "edited"
        );
        save_draft_on(&mut db, "p", "s", "", Some("edited")).unwrap();
        assert_eq!(
            db.query_row::<String, _, _>(
                "SELECT draft FROM sessions WHERE session_id='s'",
                [],
                |r| r.get(0)
            )
            .unwrap(),
            ""
        );
        db.execute("UPDATE projects SET archived=1", []).unwrap();
        assert!(save_draft_on(&mut db, "p", "s", "late", None).is_err());
    }
}

pub fn model_preferences(app: &tauri::AppHandle, mod_id: &str, session_id: &str) -> Result<(Option<crate::config::ModelSelection>, String), String> {
    let db = database::connect(app)?;
    let (provider, model, thinking): (Option<String>, Option<String>, String) = db.query_row("SELECT provider_id,model_id,thinking_level FROM sessions WHERE mod_id=?1 AND session_id=?2 AND deleted=0", params![mod_id,session_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e| e.to_string())?;
    let selection = match (provider, model) {
        (Some(provider_id), Some(model_id)) => Some(crate::config::ModelSelection { provider_id, model_id }),
        (None, None) => None,
        _ => return Err("会话模型记录不完整，请检查数据库".into()),
    };
    Ok((selection, thinking))
}

#[tauri::command]
pub fn read_session_metadata(app: tauri::AppHandle, mod_id: String, session_id: String) -> Result<serde_json::Value,String> {
    crate::storage::repository::read(&app,&mod_id)?.sessions.iter().find(|s|s.session_id==session_id).ok_or("会话不属于项目")?;
    let db = database::connect(&app)?;
    let (draft,thinking,generation):(String,String,i64) = db.query_row("SELECT draft,thinking_level,generation FROM sessions WHERE mod_id=?1 AND session_id=?2",params![mod_id,session_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e|e.to_string())?;
    let stats: Option<String> = db.query_row("SELECT payload FROM session_stats WHERE mod_id=?1 AND session_id=?2",params![mod_id,session_id],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
    let (selection, _) = model_preferences(&app, &mod_id, &session_id)?;
    let model_state = crate::config::session_model_state(&app, selection, &thinking);
    Ok(serde_json::json!({"draft":draft,"thinkingLevel":thinking,"modelState":model_state,"generation":generation,"contextUsage":stats.map(decode::<serde_json::Value>).transpose()?}))
}
pub fn steering(app: &tauri::AppHandle, input: &serde_json::Value) -> Result<(),String> {
    let mut db = database::connect(app)?;
    let tx=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
    crate::project_lifecycle::repository::ensure_writable_on(&tx,input["modId"].as_str().ok_or("引导缺少项目身份")?)?;
    tx.execute("INSERT INTO steering_inputs VALUES(?1,?2,?3,?4,?5,?6,?7,?8) ON CONFLICT(input_id) DO UPDATE SET status=excluded.status,payload=excluded.payload",params![input["inputId"].as_str(),input["modId"].as_str(),input["sessionId"].as_str(),input["taskId"].as_str(),input["generation"].as_i64(),input["submittedAt"].as_str(),input["status"].as_str(),encode(input)?]).map_err(|e|e.to_string())?;
    tx.commit().map_err(|e|e.to_string())
}
#[tauri::command]
pub fn list_steering_inputs(app: tauri::AppHandle, mod_id: String, session_id: String) -> Result<Vec<serde_json::Value>,String> {
    crate::storage::repository::read(&app,&mod_id)?.sessions.iter().find(|s|s.session_id==session_id).ok_or("会话不属于项目")?;
    let db = database::connect(&app)?;
    let mut query = db.prepare("SELECT payload FROM steering_inputs WHERE mod_id=?1 AND session_id=?2 ORDER BY rowid").map_err(|e|e.to_string())?;
    let rows = query.query_map(params![mod_id,session_id],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?;
    rows.map(|r|decode(r.map_err(|e|e.to_string())?)).collect()
}
