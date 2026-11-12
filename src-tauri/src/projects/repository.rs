use crate::storage::{database, models::ModProject, repository::read_on};
use rusqlite::{params, Connection, TransactionBehavior};

pub enum DetailChange {
    Name(String),
    InitialDraft(String),
    Description { value: String, expected: String },
}

/// 只接受项目资料业务操作，不接受完整聚合快照或任意 JSON 字段。
pub fn edit_on(
    db: &mut Connection,
    id: &str,
    expected_revision: i64,
    change: DetailChange,
) -> Result<ModProject, String> {
    let tx = db
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    crate::project_lifecycle::repository::ensure_writable_on(&tx, id)?;
    let (field, value) = match change {
        DetailChange::Name(name) => ("$.name", name),
        DetailChange::InitialDraft(value) => {
            let has_sessions: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sessions WHERE mod_id=?1 AND deleted=0)",
                    [id],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            if has_sessions {
                return Err("保存草稿必须指定会话".into());
            }
            ("$.draftRequirement", value)
        }
        DetailChange::Description { value, expected } => {
            let description: String = tx
                .query_row(
                    "SELECT json_extract(payload,'$.description') FROM projects WHERE mod_id=?1",
                    [id],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            if description != expected {
                return Err("项目描述已被修改，输入已保留，请重新打开后核对".into());
            }
            ("$.description", value.trim().to_string())
        }
    };
    if tx.execute(
        "UPDATE projects SET payload=json_set(payload,?3,?4,'$.updatedAt',?5),revision=revision+1 WHERE mod_id=?1 AND revision=?2",
        params![id, expected_revision, field, value, crate::storage::mods::now()?],
    ).map_err(|e| e.to_string())? != 1 {
        return Err("项目元数据已更新，请重新读取后操作".into());
    }
    let project = read_on(&tx, id)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(project)
}

pub fn edit(app: &tauri::AppHandle, id: &str, change: DetailChange) -> Result<ModProject, String> {
    let mut db = database::connect(app)?;
    let revision = read_on(&db, id)?.revision;
    edit_on(&mut db, id, revision, change)
}
/// 创建只写入新项目及其初始环境，会话和交付由各自业务操作建立。
pub fn create_on(db: &mut Connection, project: &ModProject) -> Result<(), String> {
    let tx = db.transaction().map_err(|e| e.to_string())?;
    if tx
        .query_row::<bool, _, _>(
            "SELECT EXISTS(SELECT 1 FROM project_removals WHERE mod_id=?1)",
            [&project.mod_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?
    {
        return Err("已移除的项目不能重新导入".into());
    }
    if !project.sessions.is_empty() {
        return Err("新项目不能包含已有会话".into());
    }
    let mut payload = serde_json::to_value(project).map_err(|e| e.to_string())?;
    payload
        .as_object_mut()
        .ok_or("项目 DTO 无效")?
        .remove("sessions");
    tx.execute(
        "INSERT INTO projects(mod_id,payload,revision) VALUES(?1,?2,0)",
        params![
            project.mod_id,
            crate::storage::repository::encode(&payload)?
        ],
    )
    .map_err(|e| e.to_string())?;
    crate::project_context::repository::initialize_on(&tx, &project.mod_id)?;
    tx.commit().map_err(|e| e.to_string())
}

pub fn create(app: &tauri::AppHandle, project: &ModProject) -> Result<(), String> {
    create_on(&mut database::connect(app)?, project)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::repository::read_on;
    use rusqlite::{params, Connection};

    fn fixture() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(include_str!("../storage/schema.sql"))
            .unwrap();
        let payload = serde_json::json!({"modId":"p","name":"原名称","gameId":"duckov","gameSkillName":"duckov-mod","revision":0,"archived":false,"description":"原描述","draftRequirement":"初始草稿","updatedAt":"before","developmentStatus":"draft","latestVersionId":"v","lastDeploymentDirectory":"D:/game","versions":[{"summary":"交付","sessionId":"s","versionId":"v","taskId":"t","createdAt":"before","skills":[],"files":[],"zipSha256":"hash"}],"installations":[{"installationId":"i","versionId":"v","targetDirectory":"D:/game","attemptedAt":"before","status":"succeeded","rollback":"notNeeded"}],"installedTargets":[{"targetDirectory":"D:/game","versionId":"v","state":"known"}]});
        db.execute(
            "INSERT INTO projects(mod_id,payload) VALUES('p',?1)",
            [payload.to_string()],
        )
        .unwrap();
        db
    }

    #[test]
    fn project_details_preserve_sessions_and_initial_draft() {
        let mut db = fixture();
        let session = serde_json::json!({"sessionId":"s","sessionFile":"history","skills":[],"readOnly":false,"name":"会话"});
        db.execute(
            "INSERT INTO sessions(mod_id,session_id,payload,draft) VALUES('p','s',?1,'会话草稿')",
            params![session.to_string()],
        )
        .unwrap();
        // 局部资料写入不应触碰会话表，即便会话存储暂不可写也应完成。
        db.execute_batch("CREATE TRIGGER reject_session_update BEFORE UPDATE ON sessions BEGIN SELECT RAISE(ABORT,'unexpected session write'); END;").unwrap();
        let renamed = edit_on(&mut db, "p", 0, DetailChange::Name("新名称".into())).unwrap();
        assert_eq!(renamed.name, "新名称");
        assert_eq!(renamed.sessions[0].name, "会话");
        assert_eq!(renamed.draft_requirement, "初始草稿");
        assert_eq!(renamed.revision, 1);
        assert_eq!(renamed.latest_version_id.as_deref(), Some("v"));
        assert_eq!(renamed.versions[0].summary, "交付");
        assert_eq!(renamed.installations[0].installation_id, "i");
        assert_eq!(
            renamed.installed_targets[0].version_id.as_deref(),
            Some("v")
        );
        assert_eq!(
            renamed.last_deployment_directory.as_deref(),
            Some("D:/game")
        );
    }

    #[test]
    fn description_conflicts_stale_revisions_and_archives_reject_without_partial_writes() {
        let mut db = fixture();
        let saved = edit_on(
            &mut db,
            "p",
            0,
            DetailChange::Description {
                value: " 新描述 ".into(),
                expected: "原描述".into(),
            },
        )
        .unwrap();
        assert_eq!(saved.description, "新描述");
        assert_eq!(saved.draft_requirement, "初始草稿");
        assert!(edit_on(&mut db, "p", 0, DetailChange::Name("陈旧名称".into())).is_err());
        assert!(edit_on(
            &mut db,
            "p",
            1,
            DetailChange::Description {
                value: "覆盖".into(),
                expected: "原描述".into()
            }
        )
        .is_err());
        assert_eq!(read_on(&db, "p").unwrap().revision, 1);
        db.execute("UPDATE projects SET archived=1 WHERE mod_id='p'", [])
            .unwrap();
        assert!(edit_on(&mut db, "p", 1, DetailChange::Name("归档名称".into())).is_err());
        let unchanged = read_on(&db, "p").unwrap();
        assert_eq!(unchanged.name, "原名称");
        assert_eq!(unchanged.description, "新描述");
        assert_eq!(unchanged.revision, 1);
    }
    #[test]
    fn initial_draft_is_local_and_rejected_after_a_session_exists() {
        let mut db = fixture();
        let saved = edit_on(
            &mut db,
            "p",
            0,
            DetailChange::InitialDraft("  原样需求\n".into()),
        )
        .unwrap();
        assert_eq!(saved.draft_requirement, "  原样需求\n");
        let session = serde_json::json!({"sessionId":"s","sessionFile":"history","skills":[],"readOnly":false,"name":"会话"});
        db.execute(
            "INSERT INTO sessions(mod_id,session_id,payload) VALUES('p','s',?1)",
            [session.to_string()],
        )
        .unwrap();
        assert!(edit_on(&mut db, "p", 1, DetailChange::InitialDraft("覆盖".into())).is_err());
        assert_eq!(read_on(&db, "p").unwrap().draft_requirement, "  原样需求\n");
        assert_eq!(read_on(&db, "p").unwrap().revision, 1);
    }
    #[test]
    fn project_creation_is_atomic_with_context_and_rejects_removed_identity() {
        let mut db = fixture();
        let mut project = read_on(&db, "p").unwrap();
        project.mod_id = "new".into();
        create_on(&mut db, &project).unwrap();
        assert_eq!(read_on(&db, "new").unwrap().name, "原名称");
        assert_eq!(
            crate::project_context::repository::read_on(&db, "new")
                .unwrap()
                .revision,
            0
        );
        project.mod_id = "removed".into();
        db.execute(
            "INSERT INTO project_removals VALUES('removed','op','now')",
            [],
        )
        .unwrap();
        assert!(create_on(&mut db, &project).is_err());
        assert!(read_on(&db, "removed").is_err());
        project.mod_id = "failed".into();
        db.execute_batch("CREATE TRIGGER reject_context BEFORE INSERT ON project_contexts BEGIN SELECT RAISE(ABORT,'context unavailable'); END;").unwrap();
        assert!(create_on(&mut db, &project).is_err());
        assert!(read_on(&db, "failed").is_err());
    }
}
