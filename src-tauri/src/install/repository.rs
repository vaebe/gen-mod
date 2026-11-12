use crate::storage::{database, repository::read_on};
use rusqlite::{params, Connection, TransactionBehavior};

pub fn remember_directory_on(
    db: &mut Connection,
    id: &str,
    revision: i64,
    directory: &str,
) -> Result<(), String> {
    let tx = db
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    crate::project_lifecycle::repository::ensure_writable_on(&tx, id)?;
    if tx.execute("UPDATE projects SET payload=json_set(payload,'$.lastDeploymentDirectory',?3,'$.updatedAt',?4),revision=revision+1 WHERE mod_id=?1 AND revision=?2",params![id,revision,directory,crate::storage::mods::now()?]).map_err(|e|e.to_string())?!=1 {return Err("项目元数据已更新，请重新读取后操作".into());}
    tx.commit().map_err(|e| e.to_string())
}

pub fn remember_directory(app: &tauri::AppHandle, id: &str, directory: &str) -> Result<(), String> {
    let mut db = database::connect(app)?;
    let revision = read_on(&db, id)?.revision;
    remember_directory_on(&mut db, id, revision, directory)
}

/// 历史尝试与当前安装状态一起提交；尚未复制的失败保留之前的已知目标。
pub fn record_attempt_on(
    db: &mut Connection,
    id: &str,
    revision: i64,
    record: &crate::install::models::InstallationRecord,
    touched: bool,
) -> Result<(), String> {
    use crate::install::models::{InstallationStatus, InstalledTarget, TargetState};
    let tx = db
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    crate::project_lifecycle::repository::ensure_writable_on(&tx, id)?;
    let mut targets = read_on(&tx, id)?.installed_targets;
    let succeeded = matches!(record.status, InstallationStatus::Succeeded);
    if succeeded || touched {
        targets.retain(|target| target.target_directory != record.target_directory);
        targets.push(InstalledTarget {
            target_directory: record.target_directory.clone(),
            version_id: if succeeded {
                Some(record.version_id.clone())
            } else {
                None
            },
            state: if succeeded {
                TargetState::Known
            } else {
                TargetState::Uncertain
            },
        });
    }
    if tx.execute("UPDATE projects SET payload=json_set(payload,'$.installations',json_insert(json_extract(payload,'$.installations'),'$[#]',json(?3)),'$.installedTargets',json(?4),'$.updatedAt',?5),revision=revision+1 WHERE mod_id=?1 AND revision=?2",params![id,revision,crate::storage::repository::encode(record)?,crate::storage::repository::encode(&targets)?,crate::storage::mods::now()?]).map_err(|e|e.to_string())?!=1 {return Err("项目元数据已更新，请重新读取后操作".into());}
    tx.commit().map_err(|e| e.to_string())
}

pub fn record_attempt(
    app: &tauri::AppHandle,
    id: &str,
    record: &crate::install::models::InstallationRecord,
    touched: bool,
) -> Result<(), String> {
    let mut db = database::connect(app)?;
    let revision = read_on(&db, id)?.revision;
    record_attempt_on(&mut db, id, revision, record, touched)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::repository::read_on;
    use rusqlite::Connection;
    #[test]
    fn preview_and_install_preserve_unrelated_data_and_current_target_meaning() {
        let mut db = Connection::open_in_memory().unwrap();
        db.execute_batch(include_str!("../storage/schema.sql"))
            .unwrap();
        let payload = serde_json::json!({"modId":"p","name":"title","gameId":"duckov","gameSkillName":"duckov-mod","revision":0,"archived":false,"description":"description","draftRequirement":"initial","updatedAt":"before","developmentStatus":"ready","versions":[],"installations":[],"installedTargets":[{"targetDirectory":"D:/game","versionId":"old","state":"known"}]});
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
        remember_directory_on(&mut db, "p", 0, "D:/game").unwrap();
        let current = read_on(&db, "p").unwrap();
        assert_eq!(
            current.last_deployment_directory.as_deref(),
            Some("D:/game")
        );
        assert_eq!(current.draft_requirement, "initial");
        assert_eq!(current.revision, 1);
        assert!(remember_directory_on(&mut db, "p", 0, "other").is_err());
        assert_eq!(
            read_on(&db, "p")
                .unwrap()
                .last_deployment_directory
                .as_deref(),
            Some("D:/game")
        );
        let mut record: crate::install::models::InstallationRecord=serde_json::from_value(serde_json::json!({"installationId":"i","versionId":"v","targetDirectory":"D:/game","attemptedAt":"now","status":"failed","rollback":"notNeeded"})).unwrap();
        record_attempt_on(&mut db, "p", 1, &record, false).unwrap();
        let untouched = read_on(&db, "p").unwrap();
        assert_eq!(
            untouched.installed_targets[0].version_id.as_deref(),
            Some("old")
        );
        assert_eq!(untouched.installations.len(), 1);
        record.installation_id = "i2".into();
        record_attempt_on(&mut db, "p", 2, &record, true).unwrap();
        let uncertain = read_on(&db, "p").unwrap();
        assert!(uncertain.installed_targets[0].version_id.is_none());
        assert!(matches!(
            uncertain.installed_targets[0].state,
            crate::install::models::TargetState::Uncertain
        ));
        record.installation_id = "i3".into();
        record.status = crate::install::models::InstallationStatus::Succeeded;
        record_attempt_on(&mut db, "p", 3, &record, true).unwrap();
        let installed = read_on(&db, "p").unwrap();
        assert_eq!(
            installed.installed_targets[0].version_id.as_deref(),
            Some("v")
        );
        assert_eq!(installed.installations.len(), 3);
        assert_eq!(installed.draft_requirement, "initial");
        assert_eq!(installed.name, "title");
        assert_eq!(installed.sessions[0].name, "session");
        assert_eq!(installed.revision, 4);
        assert!(record_attempt_on(&mut db, "p", 3, &record, true).is_err());
        assert_eq!(read_on(&db, "p").unwrap().installations.len(), 3);
        let before_recovery = read_on(&db, "p").unwrap();
        record.installation_id = "interrupted".into();
        recover_attempt_on(&mut db, "p", record.clone()).unwrap();
        recover_attempt_on(&mut db, "p", record.clone()).unwrap();
        let recovered = read_on(&db, "p").unwrap();
        assert_eq!(recovered.installations.len(), 4);
        assert_eq!(recovered.updated_at, before_recovery.updated_at);
        assert_eq!(recovered.sessions[0].name, "session");
        assert_eq!(recovered.draft_requirement, "initial");
        assert!(matches!(recovered.installed_targets[0].state, crate::install::models::TargetState::Uncertain));
        assert!(matches!(recovered.installations[3].status, crate::install::models::InstallationStatus::Failed));
        assert!(matches!(recovered.installations[3].rollback, crate::install::models::RollbackStatus::NotAttempted));
        db.execute("UPDATE projects SET archived=1 WHERE mod_id='p'", [])
            .unwrap();
        assert!(record_attempt_on(&mut db, "p", 4, &record, true).is_err());
        assert!(remember_directory_on(&mut db, "p", 4, "blocked").is_err());
        assert_eq!(read_on(&db, "p").unwrap().revision, 6);
    }
}

/// 启动恢复只补记不确定的安装结果，不更新时间或重新部署文件。
pub fn recover_attempt_on(db: &mut Connection, id: &str, mut record: crate::install::models::InstallationRecord) -> Result<(), String> {
    use crate::install::models::{InstallationStatus, RollbackStatus, InstalledTarget, TargetState};
    let tx=db.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
    crate::project_lifecycle::repository::ensure_writable_on(&tx,id)?;
    let mut project=read_on(&tx,id)?;
    if !project.installations.iter().any(|r|r.installation_id==record.installation_id) {
        record.status=InstallationStatus::Failed;
        record.rollback=RollbackStatus::NotAttempted;
        record.error=Some("上次安装记录未提交，目标文件可能已变化；未自动回退".into());
        project.installed_targets.retain(|t|t.target_directory!=record.target_directory);
        project.installed_targets.push(InstalledTarget{target_directory:record.target_directory.clone(),version_id:None,state:TargetState::Uncertain});
        project.installations.push(record);
    }
    tx.execute("UPDATE projects SET payload=json_set(payload,'$.installations',json(?2),'$.installedTargets',json(?3)),revision=revision+1 WHERE mod_id=?1",params![id,crate::storage::repository::encode(&project.installations)?,crate::storage::repository::encode(&project.installed_targets)?]).map_err(|e|e.to_string())?;
    tx.commit().map_err(|e|e.to_string())
}
