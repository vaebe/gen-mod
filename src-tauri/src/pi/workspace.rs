use std::{collections::{BTreeMap, VecDeque}, sync::{Arc, Condvar, Mutex}};
use serde_json::{json, Value};
use crate::{changes, storage::{mods, paths}};

#[derive(Clone)]
struct Lease { task: String, tool: String, operation: String, path: Option<String>, write: bool, revision: u64, before: BTreeMap<String,String> }
#[derive(Default)]
struct State {
    queue: VecDeque<String>, leases: BTreeMap<String,Lease>, cancelled: std::collections::BTreeSet<String>,
    completed_commands: BTreeMap<(String,String),u64>, observed: BTreeMap<(String,String),Option<String>>, revision: u64, fingerprint: Option<String>,
    native_operation: Option<(String, NativeOperation)>,
}
#[derive(Default)]
struct Workspace { state: Mutex<State>, wake: Condvar }
#[derive(Default)]
pub struct Workspaces(Mutex<BTreeMap<String,Arc<Workspace>>>);
#[derive(Clone, Copy, Debug)]
pub enum NativeOperation { SourceChange, TaskPrepare, ProcessConnect, ProjectLifecycle }

pub struct NativeReservation { workspace: Arc<Workspace>, operation_id: String }
impl NativeReservation {
    pub fn observe_fingerprint(&self, fingerprint: String) -> Result<u64, String> {
        let mut state = self.workspace.state.lock().map_err(|_| "工作区锁异常")?;
        if state.native_operation.as_ref().map(|v| &v.0) != Some(&self.operation_id) {
            return Err("原生项目预留已失效".into());
        }
        if state.fingerprint.as_ref() != Some(&fingerprint) { state.revision += 1; }
        state.fingerprint = Some(fingerprint);
        Ok(state.revision)
    }
}
impl Drop for NativeReservation {
    fn drop(&mut self) {
        if let Ok(mut state) = self.workspace.state.lock() {
            if state.native_operation.as_ref().map(|v| &v.0) == Some(&self.operation_id) {
                state.native_operation = None;
                self.workspace.wake.notify_all();
            }
        }
    }
}
impl Workspaces {
    pub fn forget_project(&self,id: &str) -> Result<(),String> {
        self.0.lock().map_err(|_| "工作区注册表锁异常")?.remove(id);
        Ok(())
    }
    fn workspace(&self, id: &str) -> Result<Arc<Workspace>, String> {
        Ok(self.0.lock().map_err(|_| "工作区注册表锁异常")?.entry(id.into()).or_default().clone())
    }
    pub fn try_reserve_native(&self, mod_id: &str, operation: NativeOperation) -> Result<NativeReservation, String> {
        let workspace = self.workspace(mod_id)?;
        let mut state = workspace.state.lock().map_err(|_| "工作区锁异常")?;
        if state.native_operation.is_some() || !state.leases.is_empty() || !state.queue.is_empty() {
            return Err("当前项目正在操作，请完成后重试".into());
        }
        let operation_id = uuid::Uuid::new_v4().to_string();
        state.native_operation = Some((operation_id.clone(), operation));
        drop(state);
        Ok(NativeReservation { workspace, operation_id })
    }
    /// 排队线程不持会话槽锁；FIFO 不允许新读取越过等待写入。
    pub fn acquire(&self, app: &tauri::AppHandle, context: &super::tasks::TaskContext, request: &Value) -> Result<Value,String> {
        crate::project_lifecycle::repository::ensure_writable(app,&context.mod_id)?;
        let id = request["leaseId"].as_str().ok_or("租约缺少 ID")?.to_owned(); paths::validate_id(&id)?;
        let tool = request["toolCallId"].as_str().ok_or("租约缺少工具 ID")?.to_owned();
        let operation = request["operation"].as_str().ok_or("租约缺少操作")?.to_owned();
        if !matches!(operation.as_str(),"read"|"grep"|"find"|"ls"|"write"|"edit"|"run_command"|"prepare_delivery"|"snapshot") { return Err("未知工作区操作".into()); }
        let write = matches!(operation.as_str(),"write"|"edit"|"run_command"|"prepare_delivery");
        let path = request["path"].as_str().map(str::to_owned);
        if matches!(operation.as_str(),"write"|"edit") && path.is_none() { return Err("文件操作缺少路径".into()); }
        if let Some(path) = &path { if path != "." { checked_file(context,path)?; } }
        let workspace = self.workspace(&context.mod_id)?;
        let mut state = workspace.state.lock().map_err(|_| "工作区锁异常")?;
        if state.native_operation.is_some() { return Err("当前项目正在执行原生操作，请稍后重试".into()); }
        if state.cancelled.remove(&id) { return Err("工作区等待已取消".into()); }
        let observed_revision = state.revision;
        state.queue.push_back(id.clone());
        let deadline = std::time::Instant::now()+std::time::Duration::from_secs(25);
        loop {
            if state.cancelled.remove(&id) { state.queue.retain(|v| v != &id); workspace.wake.notify_all(); return Err("工作区等待已取消".into()); }
            if std::time::Instant::now() >= deadline {
                let holders: Vec<Value> = state.leases.iter().map(|(id,lease)|json!({"leaseId":id,"taskId":lease.task,"toolCallId":lease.tool,"operation":lease.operation,"write":lease.write,"sourceRevision":lease.revision})).collect();
                let queued = state.queue.len();
                state.queue.retain(|v| v != &id); workspace.wake.notify_all();
                return Err(format!("工作区等待超时（25000ms），工具尚未开始 · leaseId {id} · operation {operation} · 排队 {queued} · 占用租约 {}",json!(holders)));
            }
            if state.queue.front() == Some(&id) && (state.leases.is_empty() || (!write && state.leases.values().all(|v| !v.write))) { break; }
            state = workspace.wake.wait_timeout(state,std::time::Duration::from_millis(250)).map_err(|_| "工作区等待锁异常")?.0;
        }
        state.queue.pop_front();
        let result = (|| {
            crate::project_lifecycle::repository::ensure_writable(app,&context.mod_id)?;
            let fingerprint = fingerprint(app,context)?;
            if state.fingerprint.as_ref().is_some_and(|v| v != &fingerprint) { state.revision += 1; }
            state.fingerprint = Some(fingerprint);
            if operation=="run_command" && state.revision!=observed_revision { return Err("排队期间源码已变化；请重新读取相关源码并检查命令后重试".into()); }
            if matches!(operation.as_str(),"write"|"edit") {
                let path = path.as_ref().ok_or("文件路径缺失")?;
                let actual = hash(context,path)?;
                match state.observed.get(&(context.task_id.clone(),path.to_lowercase())) {
                    Some(old) if old != &actual => return Err("源码已被其他操作修改；请重新 read 后再修改".into()),
                    None if actual.is_some() => return Err("覆盖已有源码前必须先 read 当前文件".into()),
                    _ => {}
                }
            }
            let revision = state.revision;
            let before = if write { source_manifest(app,context)? } else { BTreeMap::new() };
            state.leases.insert(id.clone(),Lease {before,task:context.task_id.clone(),tool,operation,path,write,revision});
            Ok(json!({"leaseId":id,"sourceRevision":revision}))
        })();
        workspace.wake.notify_all();
        result
    }
    pub fn release(&self, app: &tauri::AppHandle, context: &super::tasks::TaskContext, request: &Value) -> Result<Value,String> {
        let workspace = self.workspace(&context.mod_id)?;
        let mut state = workspace.state.lock().map_err(|_| "工作区锁异常")?;
        let id = request["leaseId"].as_str().ok_or("缺少租约 ID")?;
        let lease = state.leases.get(id).ok_or("租约不存在")?.clone();
        if lease.task != context.task_id || request["toolCallId"] != lease.tool { return Err("租约不属于当前工具".into()); }
        let result = (|| {
            if lease.write {
                let fingerprint = fingerprint(app,context)?;
                if state.fingerprint.as_ref() != Some(&fingerprint) { state.revision += 1; }
                state.fingerprint = Some(fingerprint);
            }
            if matches!(lease.operation.as_str(),"read"|"write"|"edit") && request["success"] == true {
                if let Some(path) = &lease.path { state.observed.insert((context.task_id.clone(),path.to_lowercase()),hash(context,path)?); }
            }
            if lease.write {
                changes::refresh(app,context)?;
                let directory = paths::project_dir(app,&context.mod_id)?.join("logs").join(&context.task_id);
                use std::io::Write;
                let mut file = std::fs::OpenOptions::new().append(true).create(true).open(directory.join("workspace-operations.jsonl")).map_err(|e| e.to_string())?;
                let after=source_manifest(app,context)?;
                let mut keys: std::collections::BTreeSet<String> = lease.before.keys().chain(after.keys()).cloned().collect();
                keys.retain(|key|lease.before.get(key)!=after.get(key));
                let changes: Vec<Value> = keys.into_iter().map(|path|json!({"path":path,"beforeHash":lease.before.get(&path),"afterHash":after.get(&path)})).collect();
                writeln!(file,"{}",json!({"fileChanges":changes,"toolCallId":lease.tool,"operation":lease.operation,"path":lease.path,"beforeRevision":lease.revision,"sourceRevision":state.revision,"success":request["success"],"timestamp":mods::now()?})).map_err(|e| e.to_string())?;
            }
            if lease.operation=="run_command" { let revision = state.revision; state.completed_commands.insert((context.task_id.clone(),lease.tool.clone()),revision); }
            Ok(json!({"sourceRevision":state.revision}))
        })();
        state.leases.remove(id); workspace.wake.notify_all(); result
    }
    pub fn cancel(&self, mod_id: &str, id: &str) -> Result<(),String> {
        let workspace = self.workspace(mod_id)?;
        let mut state = workspace.state.lock().map_err(|_| "工作区锁异常")?;
        state.queue.retain(|v|v!=id); state.leases.remove(id); state.cancelled.insert(id.into()); workspace.wake.notify_all(); Ok(())
    }
    /// 必须先回收 Job 后调用；释放任务持有的租约并取消其等待。
    pub fn cleanup(&self, mod_id: &str, task_id: &str, waiting: &[String]) -> Result<(),String> {
        let workspace = self.workspace(mod_id)?;
        let mut state = workspace.state.lock().map_err(|_| "工作区锁异常")?;
        state.leases.retain(|_,lease|lease.task!=task_id);
        for id in waiting { state.queue.retain(|v|v!=id); state.cancelled.insert(id.clone()); }
        workspace.wake.notify_all(); Ok(())
    }
    pub fn validate_command(&self, context: &super::tasks::TaskContext, receipt: &super::tasks::CommandReceipt) -> Result<(),String> {
        let workspace = self.workspace(&context.mod_id)?;
        let state = workspace.state.lock().map_err(|_| "工作区锁异常")?;
        let revision = state.completed_commands.get(&(context.task_id.clone(),receipt.tool_call_id.clone())).copied()
            .or_else(||state.leases.values().find(|v|v.task==context.task_id && v.tool==receipt.tool_call_id && v.operation=="prepare_delivery").map(|v|v.revision));
        if revision!=Some(receipt.source_revision) { return Err("命令回执缺少原生租约 revision".into()); } Ok(())
    }
    pub fn validate_delivery(&self, app: &tauri::AppHandle, context: &super::tasks::TaskContext, lease_id: &str, revision: u64) -> Result<(),String> {
        let workspace = self.workspace(&context.mod_id)?;
        let state = workspace.state.lock().map_err(|_| "工作区锁异常")?;
        let lease = state.leases.get(lease_id).ok_or("交付缺少活动租约")?;
        if lease.task != context.task_id || lease.operation != "prepare_delivery" || lease.revision != revision || state.fingerprint.as_ref()!=Some(&fingerprint(app,context)?) { return Err("交付源码 revision 已变化，必须重新构建".into()); }
        Ok(())
    }
}
fn checked_file(context: &super::tasks::TaskContext, path: &str) -> Result<std::path::PathBuf,String> {
    let root = std::path::Path::new(&context.source_directory);
    let file = root.join(changes::relative_path(path)?);
    for ancestor in file.ancestors() { if ancestor.exists() { paths::reject_redirect(ancestor)?; } if ancestor == root { break; } }
    Ok(file)
}
fn hash(context: &super::tasks::TaskContext, path: &str) -> Result<Option<String>,String> {
    let file = checked_file(context,path)?;
    if !file.exists() { return Ok(None); }
    changes::file_hash(&file).map(Some)
}
fn source_manifest(app: &tauri::AppHandle, context: &super::tasks::TaskContext) -> Result<BTreeMap<String,String>,String> {
    mods::read_project(app,&context.mod_id)?;
    let mut excludes=vec![".delivery".into()];
    excludes.extend(crate::skills::registry::task_views(app,context)?.into_iter().flat_map(|s|s.definition.exclude_paths));
    changes::source_manifest(std::path::Path::new(&context.source_directory),&excludes)
}
fn fingerprint(app: &tauri::AppHandle, context: &super::tasks::TaskContext) -> Result<String,String> {
    mods::read_project(app,&context.mod_id)?;
    let mut excludes = vec![".delivery".into()];
    excludes.extend(crate::skills::registry::task_views(app,context)?.into_iter().flat_map(|s|s.definition.exclude_paths));
    changes::source_fingerprint(std::path::Path::new(&context.source_directory),&excludes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_reservation_is_project_scoped_and_released_on_drop() {
        let workspaces = Workspaces::default();
        let first = workspaces.try_reserve_native("one", NativeOperation::SourceChange).unwrap();
        assert!(workspaces.try_reserve_native("one", NativeOperation::TaskPrepare).is_err());
        let other = workspaces.try_reserve_native("two", NativeOperation::TaskPrepare).unwrap();
        drop(first);
        assert!(workspaces.try_reserve_native("one", NativeOperation::TaskPrepare).is_ok());
        drop(other);
    }

    #[test]
    fn concurrent_source_and_task_reservations_have_one_winner() {
        let workspaces = Arc::new(Workspaces::default());
        let start = Arc::new(std::sync::Barrier::new(3));
        let finish = Arc::new(std::sync::Barrier::new(3));
        let handles: Vec<_> = [NativeOperation::SourceChange, NativeOperation::TaskPrepare].into_iter().map(|operation| {
            let (workspaces, start, finish) = (workspaces.clone(), start.clone(), finish.clone());
            std::thread::spawn(move || {
                start.wait();
                let reservation = workspaces.try_reserve_native("one", operation);
                let won = reservation.is_ok();
                finish.wait();
                drop(reservation);
                won
            })
        }).collect();
        start.wait();
        finish.wait();
        assert_eq!(handles.into_iter().map(|handle| handle.join().unwrap()).filter(|won| *won).count(), 1);
    }

    #[test]
    fn native_reservation_rejects_an_existing_source_lease() {
        let workspaces = Workspaces::default();
        let workspace = workspaces.workspace("one").unwrap();
        workspace.state.lock().unwrap().leases.insert("lease".into(), Lease {
            task: "task".into(), tool: "tool".into(), operation: "read".into(), path: None,
            write: false, revision: 0, before: BTreeMap::new(),
        });
        assert!(workspaces.try_reserve_native("one", NativeOperation::SourceChange).is_err());
    }
    #[test]
    fn process_connect_and_lifecycle_reserve_before_each_others_critical_phase(){
        for first in [NativeOperation::ProcessConnect,NativeOperation::ProjectLifecycle] {
            let workspaces=Arc::new(Workspaces::default());
            let checked=Arc::new(std::sync::Barrier::new(2));let release=Arc::new(std::sync::Barrier::new(2));
            let worker={let (workspaces,checked,release)=(workspaces.clone(),checked.clone(),release.clone());std::thread::spawn(move||{
                let _reservation=workspaces.try_reserve_native("p",first).unwrap();checked.wait();release.wait();
            })};
            checked.wait();
            let other=match first {NativeOperation::ProcessConnect=>NativeOperation::ProjectLifecycle,_=>NativeOperation::ProcessConnect};
            assert!(workspaces.try_reserve_native("p",other).is_err());
            assert!(workspaces.try_reserve_native("other",other).is_ok());
            release.wait();worker.join().unwrap();assert!(workspaces.try_reserve_native("p",other).is_ok());
        }
    }
}
