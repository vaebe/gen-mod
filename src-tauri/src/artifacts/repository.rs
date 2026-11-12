use crate::{artifacts::models::ArtifactVersion, storage::repository::encode};
use rusqlite::{params, Transaction};

/// 加入调用方事务，与交付锚点及任务登记一起提交，只写交付所属字段。
pub fn register_on(
    tx: &Transaction<'_>,
    id: &str,
    revision: i64,
    version: &ArtifactVersion,
    updated: &str,
) -> Result<(), String> {
    crate::project_lifecycle::repository::ensure_writable_on(tx, id)?;
    if tx.execute("UPDATE projects SET payload=json_set(payload,'$.versions',json_insert(json_extract(payload,'$.versions'),'$[#]',json(?3)),'$.latestVersionId',?4,'$.developmentStatus','ready','$.updatedAt',?5),revision=revision+1 WHERE mod_id=?1 AND revision=?2",params![id,revision,encode(version)?,version.version_id,updated]).map_err(|e|e.to_string())? != 1 {
        return Err("项目元数据已更新，请重新读取后操作".into());
    }
    Ok(())
}

/// 删除包不更改安装历史或游戏目录当前状态，交付锚点由调用方保留。
pub fn delete_on(
    tx: &Transaction<'_>,
    id: &str,
    revision: i64,
    version_id: &str,
    updated: &str,
) -> Result<(), String> {
    crate::project_lifecycle::repository::ensure_writable_on(tx, id)?;
    let mut project = crate::storage::repository::read_on(tx, id)?;
    project
        .versions
        .retain(|version| version.version_id != version_id);
    if project.latest_version_id.as_deref() == Some(version_id) {
        project.latest_version_id = project
            .versions
            .last()
            .map(|version| version.version_id.clone());
    }
    if project.versions.is_empty()
        && matches!(
            project.development_status,
            crate::storage::models::DevelopmentStatus::Ready
        )
    {
        project.development_status = crate::storage::models::DevelopmentStatus::Generated;
    }
    if tx.execute("UPDATE projects SET payload=json_set(payload,'$.versions',json(?3),'$.latestVersionId',?4,'$.developmentStatus',json(?5),'$.updatedAt',?6),revision=revision+1 WHERE mod_id=?1 AND revision=?2",params![id,revision,encode(&project.versions)?,project.latest_version_id,encode(&project.development_status)?,updated]).map_err(|e|e.to_string())?!=1 {return Err("项目元数据已更新，请重新读取后操作".into());}
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::repository::read_on;
    use rusqlite::Connection;
    #[test]
    fn delivery_preserves_session_draft_and_project_details() {
        let mut db = Connection::open_in_memory().unwrap();
        db.execute_batch(include_str!("../storage/schema.sql"))
            .unwrap();
        let payload = serde_json::json!({"modId":"p","name":"title","gameId":"duckov","gameSkillName":"duckov-mod","revision":0,"archived":false,"description":"description","draftRequirement":"initial","updatedAt":"before","developmentStatus":"running","versions":[],"installations":[{"installationId":"i","versionId":"old","targetDirectory":"D:/game","attemptedAt":"before","status":"succeeded","rollback":"notNeeded"}],"installedTargets":[{"targetDirectory":"D:/game","versionId":"old","state":"known"}]});
        db.execute(
            "INSERT INTO projects(mod_id,payload) VALUES('p',?1)",
            [payload.to_string()],
        )
        .unwrap();
        let session = serde_json::json!({"sessionId":"s","sessionFile":"history","skills":[],"readOnly":false,"name":"session"});
        db.execute(
            "INSERT INTO sessions(mod_id,session_id,payload,draft) VALUES('p','s',?1,'draft')",
            [session.to_string()],
        )
        .unwrap();
        db.execute_batch("CREATE TRIGGER reject_session_update BEFORE UPDATE ON sessions BEGIN SELECT RAISE(ABORT,'unexpected session write'); END;").unwrap();
        let version = serde_json::from_value(serde_json::json!({"summary":"delivery","sessionId":"s","versionId":"v","taskId":"t","createdAt":"now","skills":[],"files":[],"zipSha256":"hash"})).unwrap();
        // 调用方后续锚点或任务写入失败时，交付字段必须随同回退。
        let tx = db.transaction().unwrap();
        register_on(&tx, "p", 0, &version, "now").unwrap();
        tx.rollback().unwrap();
        assert!(read_on(&db, "p").unwrap().versions.is_empty());
        assert_eq!(read_on(&db, "p").unwrap().revision, 0);
        let tx = db.transaction().unwrap();
        register_on(&tx, "p", 0, &version, "now").unwrap();
        tx.commit().unwrap();
        let current = read_on(&db, "p").unwrap();
        assert_eq!(current.versions.len(), 1);
        assert_eq!(current.latest_version_id.as_deref(), Some("v"));
        assert_eq!(current.draft_requirement, "initial");
        assert_eq!(current.description, "description");
        assert_eq!(current.sessions[0].name, "session");
        assert_eq!(current.revision, 1);
        let tx = db.transaction().unwrap();
        assert!(delete_on(&tx, "p", 0, "v", "deleted").is_err());
        tx.rollback().unwrap();
        let tx = db.transaction().unwrap();
        delete_on(&tx, "p", 1, "v", "deleted").unwrap();
        tx.commit().unwrap();
        let removed = read_on(&db, "p").unwrap();
        assert!(removed.versions.is_empty());
        assert!(removed.latest_version_id.is_none());
        assert!(matches!(
            removed.development_status,
            crate::storage::models::DevelopmentStatus::Generated
        ));
        assert_eq!(removed.draft_requirement, "initial");
        assert_eq!(removed.revision, 2);
        assert_eq!(removed.installations[0].installation_id, "i");
        assert_eq!(
            removed.installed_targets[0].version_id.as_deref(),
            Some("old")
        );
        db.execute("UPDATE projects SET archived=1 WHERE mod_id='p'", [])
            .unwrap();
        let tx = db.transaction().unwrap();
        assert!(register_on(&tx, "p", 2, &version, "blocked").is_err());
        tx.rollback().unwrap();
        assert_eq!(read_on(&db, "p").unwrap().revision, 2);
        assert_eq!(
            db.query_row::<String, _, _>("SELECT draft FROM sessions WHERE mod_id='p'", [], |r| r
                .get(0))
                .unwrap(),
            "draft"
        );
    }
}
