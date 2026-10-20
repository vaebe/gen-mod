use super::repository::*;
use crate::storage::{paths,repository::{decode,encode}};
use rusqlite::{Connection,params};
use serde::{Serialize,Deserialize};
use std::{fs,path::{Path,PathBuf}};

#[derive(Serialize,Deserialize,Clone)]
#[serde(rename_all="camelCase")]
pub struct Removal {
    pub mod_id:String,
    pub operation_id:String,
    pub revision:i64,
    pub directory_identity:String,
}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct DeleteProjectResult { pub mod_id:String,pub removed:bool,pub cleanup_pending:bool,pub message:Option<String> }

fn exists(path:&Path)->Result<bool,String> {
    match fs::symlink_metadata(path) {Ok(_)=>Ok(true),Err(e) if e.kind()==std::io::ErrorKind::NotFound=>Ok(false),Err(e)=>Err(e.to_string())}
}
pub fn controlled_paths(root:&Path,id:&str,operation:&str)->Result<(PathBuf,PathBuf),String> {
    paths::validate_id(id)?;paths::validate_id(operation)?;
    for ancestor in root.ancestors(){paths::reject_redirect(ancestor)?;}
    let root=root.canonicalize().map_err(|e|e.to_string())?;
    let original=root.join(id);let temporary=root.join(format!(".delete-project-{operation}"));
    for path in [&original,&temporary] {
        if path.parent()!=Some(root.as_path()){return Err("项目删除目录越界".into());}
        if exists(path)? {paths::reject_redirect(path)?;if !path.is_dir(){return Err("项目删除目标不是目录".into());}}
    }
    Ok((original,temporary))
}
pub fn directory_identity(path:&Path)->Result<String,String> {
    paths::reject_redirect(path)?;
    let metadata=fs::symlink_metadata(path).map_err(|e|e.to_string())?;
    #[cfg(windows)] {use std::os::windows::fs::MetadataExt;Ok(format!("created:{}",metadata.creation_time()))}
    #[cfg(not(windows))] {use std::os::unix::fs::MetadataExt;Ok(format!("{}:{}",metadata.dev(),metadata.ino()))}
}
fn verify_directory(path:&Path,removal:&Removal)->Result<(),String> {
    if directory_identity(path)?!=removal.directory_identity {return Err("项目目录身份发生变化，保留文件等待核对".into());}
    Ok(())
}
pub fn journal_on(db:&Connection,removal:&Removal,status:&str)->Result<(),String> {
    db.execute("INSERT INTO file_operations VALUES(?1,?2,'project_delete',?3,?4) ON CONFLICT(operation_id) DO UPDATE SET status=excluded.status,payload=excluded.payload",params![removal.operation_id,removal.mod_id,status,encode(removal)?]).map_err(|e|e.to_string())?;
    Ok(())
}
fn verify_project(db:&Connection,removal:&Removal)->Result<(),String> {
    let revision:i64=db.query_row("SELECT revision FROM projects WHERE mod_id=?1",[&removal.mod_id],|r|r.get(0)).map_err(|e|e.to_string())?;
    if revision!=removal.revision{return Err("项目元数据身份已变化，不能自动恢复".into());}
    Ok(())
}
fn collect_tree(path:&Path,entries:&mut Vec<PathBuf>)->Result<(),String> {
    paths::reject_redirect(path)?;
    let metadata=fs::symlink_metadata(path).map_err(|e|e.to_string())?;
    if metadata.is_dir(){for item in fs::read_dir(path).map_err(|e|e.to_string())? {collect_tree(&item.map_err(|e|e.to_string())?.path(),entries)?;}}
    else if !metadata.is_file(){return Err("删除目录含特殊文件，保留等待核对".into());}
    entries.push(path.to_path_buf());Ok(())
}
// 先完整核查，再逐项清理；不沿重解析点递归到项目以外。
pub fn cleanup(root:&Path,removal:&Removal)->Result<(),String> {
    let (_,temporary)=controlled_paths(root,&removal.mod_id,&removal.operation_id)?;
    if !exists(&temporary)?{return Ok(());}
    verify_directory(&temporary,removal)?;
    let mut entries=Vec::new();collect_tree(&temporary,&mut entries)?;
    for path in entries {
        paths::reject_redirect(&path)?;
        if path.is_dir(){fs::remove_dir(&path)}else{fs::remove_file(&path)}.map_err(|e|e.to_string())?;
    }
    Ok(())
}
pub fn recover_one(db:&mut Connection,root:&Path,removal:&Removal)->Result<(),String> {
    let result=(||->Result<(),String>{
        let (original,temporary)=controlled_paths(root,&removal.mod_id,&removal.operation_id)?;
        let removed:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM project_removals WHERE mod_id=?1 AND operation_id=?2)",params![removal.mod_id,removal.operation_id],|r|r.get(0)).map_err(|e|e.to_string())?;
        if removed {cleanup(root,removal)?;return journal_on(db,removal,"done");}
        verify_project(db,removal)?;
        if exists(&temporary)? {
            verify_directory(&temporary,removal)?;
            if exists(&original)?{return Err("项目恢复路径已被占用，保留两份目录等待核对".into());}
            fs::rename(&temporary,&original).map_err(|e|e.to_string())?;
        }
        verify_directory(&original,removal)?;
        journal_on(db,removal,"interrupted")
    })();
    if let Err(error)=result {
        journal_on(db,removal,"requires_review")?;
        db.execute("UPDATE file_operations SET payload=json_set(payload,'$.reason',?2) WHERE operation_id=?1",params![removal.operation_id,error]).map_err(|e|e.to_string())?;
        return Err(error);
    }
    Ok(())
}
pub fn begin_delete(db:&mut Connection,root:&Path,id:&str,expected:i64)->Result<Removal,String> {
    ensure_idle_on(db,id)?;
    if has_unresolved_removal_on(db,id)?{return Err("项目移除恢复尚未完成".into());}
    let project=crate::storage::repository::read_on(db,id)?;
    if project.revision!=expected{return Err("项目已变化，请重新加载后操作".into());}
    let pending:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM file_operations WHERE mod_id=?1 AND status IN ('pending','requires_review'))",[id],|r|r.get(0)).map_err(|e|e.to_string())?;
    if pending{return Err("项目有尚未恢复的文件操作，暂不能删除".into());}
    let operation_id=uuid::Uuid::new_v4().to_string();
    let (original,temporary)=controlled_paths(root,id,&operation_id)?;
    if exists(&temporary)?{return Err("删除临时目录已存在，未覆盖".into());}
    let removal=Removal{mod_id:id.into(),operation_id,revision:expected,directory_identity:directory_identity(&original)?};
    journal_on(db,&removal,"pending")?;
    if let Err(error)=fs::rename(&original,&temporary) {
        let recovery=recover_one(db,root,&removal);
        return Err(format!("项目目录移除失败：{error}{}",recovery.err().map(|e|format!("；恢复待处理：{e}")).unwrap_or_default()));
    }
    if let Err(error)=delete_private_rows_on(db,id,&removal.operation_id,&crate::storage::mods::now()?) {
        let recovery=recover_one(db,root,&removal);
        return Err(format!("项目数据删除未提交：{error}{}",recovery.err().map(|e|format!("；恢复待处理：{e}")).unwrap_or_default()));
    }
    Ok(removal)
}
pub fn finish_delete(db:&Connection,root:&Path,removal:&Removal)->DeleteProjectResult {
    let error=cleanup(root,removal).and_then(|_|journal_on(db,removal,"done")).err();
    DeleteProjectResult{mod_id:removal.mod_id.clone(),removed:true,cleanup_pending:error.is_some(),message:error.map(|e|format!("项目已移除，文件清理未完成：{e}"))}
}
pub fn recover_pending(app:&tauri::AppHandle)->Result<(),String> {
    let mut db=crate::storage::database::connect(app)?;
    let removals={let mut query=db.prepare("SELECT operation_id,mod_id,payload FROM file_operations WHERE kind='project_delete' AND status IN ('pending','requires_review')").map_err(|e|e.to_string())?;
        let rows=query.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?))).map_err(|e|e.to_string())?;
        rows.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?};
    for (operation,id,payload) in removals {
        let removal=match decode::<Removal>(payload) {
            Ok(value) if value.operation_id==operation && value.mod_id==id=>value,
            _=>{db.execute("UPDATE file_operations SET status='requires_review' WHERE operation_id=?1",[operation]).map_err(|e|e.to_string())?;continue;},
        };
        if let Err(error)=recover_one(&mut db,&paths::mods_root(app)?,&removal) {
            let blocked:String=db.query_row("SELECT status FROM file_operations WHERE operation_id=?1",[&removal.operation_id],|r|r.get(0)).map_err(|e|e.to_string())?;
            if blocked!="requires_review"{return Err(error);}
        }
    }
    Ok(())
}
