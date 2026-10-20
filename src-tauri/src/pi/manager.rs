use std::{collections::BTreeMap, sync::{Arc, Mutex}};

/// 注册表仅保护槽查找；源码操作与进程生命周期各持目标槽锁。
pub struct Registry<T: Default>(Mutex<BTreeMap<(String,String), Arc<Mutex<T>>>>);
impl<T: Default> Default for Registry<T> { fn default() -> Self { Self(Mutex::new(BTreeMap::new())) } }
impl<T: Default> Registry<T> {
    pub fn slot(&self, mod_id: &str, session_id: &str) -> Result<Arc<Mutex<T>>, String> {
        crate::storage::paths::validate_id(mod_id)?;
        crate::storage::paths::validate_id(session_id)?;
        let mut slots = self.0.lock().map_err(|_| "会话注册表锁异常")?;
        Ok(slots.entry((mod_id.into(),session_id.into())).or_default().clone())
    }
    pub fn all(&self) -> Result<Vec<Arc<Mutex<T>>>, String> {
        Ok(self.0.lock().map_err(|_| "会话注册表锁异常")?.values().cloned().collect())
    }
    pub fn for_project(&self, mod_id: &str) -> Result<Vec<Arc<Mutex<T>>>,String> {
        Ok(self.0.lock().map_err(|_| "会话注册表锁异常")?.iter().filter(|((id,_),_)|id==mod_id).map(|(_,slot)|slot.clone()).collect())
    }
    pub fn remove_project(&self, mod_id: &str) -> Result<(),String> {
        self.0.lock().map_err(|_| "会话注册表锁异常")?.retain(|(id,_),_|id!=mod_id);
        Ok(())
    }
}
