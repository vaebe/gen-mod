/// Job Object 生命周期与 pi 相同，关闭句柄清理包括命令工具在内的后代。
#[cfg(windows)]
pub struct ProcessJob(usize);
#[derive(serde::Serialize)]
pub struct JobProcess { pub pid: u32, pub image: Option<String>, pub query_error: Option<String> }
#[cfg(windows)]
impl ProcessJob {
    /// 返回具体 PID；pi 的常驻控制台也在 Job 内，不能用进程总数判断命令结束。
    pub fn process_ids(&self) -> Result<Vec<u32>,String> {
        use windows_sys::Win32::System::JobObjects::*;
        use windows_sys::Win32::Foundation::ERROR_MORE_DATA;
        let mut capacity = 32usize;
        loop {
            // usize 缓冲保证 ULONG_PTR 对齐，头部为两个 DWORD，后接变长 PID 数组。
            let header = std::mem::offset_of!(JOBOBJECT_BASIC_PROCESS_ID_LIST, ProcessIdList);
            let mut buffer = vec![0usize; header.div_ceil(std::mem::size_of::<usize>()) + capacity];
            unsafe {
                let ok = QueryInformationJobObject(self.0 as _,JobObjectBasicProcessIdList,buffer.as_mut_ptr() as _,std::mem::size_of_val(buffer.as_slice()) as u32,std::ptr::null_mut());
                let info = &*(buffer.as_ptr() as *const JOBOBJECT_BASIC_PROCESS_ID_LIST);
                let count = info.NumberOfProcessIdsInList as usize;
                if ok != 0 && count <= capacity && count == info.NumberOfAssignedProcesses as usize {
                    let ids = std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>().add(header).cast::<usize>(),count);
                    return Ok(ids.iter().map(|id| *id as u32).collect());
                }
                let error = std::io::Error::last_os_error();
                if ok == 0 && error.raw_os_error() != Some(ERROR_MORE_DATA as i32) { return Err(format!("查询 Job 进程列表失败：{error}")); }
                capacity = capacity.max(info.NumberOfAssignedProcesses as usize).checked_mul(2).filter(|v| *v <= 65_536).ok_or("Job 进程列表过大，无法确认命令后代")?;
            }
        }
    }
    pub fn describe_processes(&self, ids: &[u32]) -> Vec<JobProcess> {
        use windows_sys::Win32::{Foundation::CloseHandle, System::Threading::*};
        ids.iter().map(|pid| unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION,0,*pid);
            if handle.is_null() { return JobProcess {pid:*pid,image:None,query_error:Some(std::io::Error::last_os_error().to_string())}; }
            let mut name = vec![0u16;32_768];
            let mut length = name.len() as u32;
            let info = if QueryFullProcessImageNameW(handle,0,name.as_mut_ptr(),&mut length) != 0 {
                JobProcess {pid:*pid,image:Some(String::from_utf16_lossy(&name[..length as usize])),query_error:None}
            } else { JobProcess {pid:*pid,image:None,query_error:Some(std::io::Error::last_os_error().to_string())} };
            CloseHandle(handle); info
        }).collect()
    }
    /// 只回收此 Job 内、基线以外的进程；调用方继续查询 Job 确认全部退出。
    pub fn terminate_descendants(&self, baseline: &[u32]) -> Result<Vec<u32>,String> {
        use windows_sys::Win32::{Foundation::{CloseHandle,ERROR_INVALID_PARAMETER,WAIT_OBJECT_0}, System::{JobObjects::IsProcessInJob,Threading::*}};
        let remaining: Vec<u32> = self.process_ids()?.into_iter().filter(|pid| !baseline.contains(pid)).collect();
        let mut terminated = Vec::new();
        for pid in remaining {
            unsafe {
                let handle = OpenProcess(PROCESS_TERMINATE|PROCESS_QUERY_LIMITED_INFORMATION|PROCESS_SYNCHRONIZE,0,pid);
                if handle.is_null() {
                    let error = std::io::Error::last_os_error();
                    if error.raw_os_error()==Some(ERROR_INVALID_PARAMETER as i32) { continue; }
                    return Err(format!("无法回收命令后代 PID {pid}：{error}"));
                }
                let result = (|| -> Result<bool,String> {
                    let mut member = 0;
                    if IsProcessInJob(handle,self.0 as _,&mut member)==0 { return Err(format!("无法确认 PID {pid} 所属 Job：{}",std::io::Error::last_os_error())); }
                    // PID 可能已退出或被复用，不操作已离开本 Job 的进程。
                    if member==0 || WaitForSingleObject(handle,0)==WAIT_OBJECT_0 { return Ok(false); }
                    if TerminateProcess(handle,1)==0 {
                        let error = std::io::Error::last_os_error();
                        if WaitForSingleObject(handle,0)!=WAIT_OBJECT_0 { return Err(format!("终止命令后代 PID {pid} 失败：{error}")); }
                    }
                    Ok(true)
                })();
                CloseHandle(handle);
                if result? { terminated.push(pid); }
            }
        }
        Ok(terminated)
    }
    /// 关闭 Job 前等待后代全部退出，租约不能早于文件写入进程的终止释放。
    pub fn terminate_and_wait(&self) -> Result<(),String> {
        use windows_sys::Win32::System::JobObjects::*;
        unsafe {
            if TerminateJobObject(self.0 as _,1)==0 { return Err(std::io::Error::last_os_error().to_string()); }
            let deadline=std::time::Instant::now()+std::time::Duration::from_secs(10);
            loop {
                let mut info: JOBOBJECT_BASIC_ACCOUNTING_INFORMATION=std::mem::zeroed();
                if QueryInformationJobObject(self.0 as _,JobObjectBasicAccountingInformation,&mut info as *mut _ as _,std::mem::size_of_val(&info) as u32,std::ptr::null_mut())==0 { return Err(std::io::Error::last_os_error().to_string()); }
                if info.ActiveProcesses==0 { return Ok(()); }
                if std::time::Instant::now()>=deadline { return Err("后代进程尚未退出，保留源码租约".into()); }
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
        }
    }
    pub fn assign(child: &std::process::Child) -> Result<Self, String> {
        use std::{mem::size_of, os::windows::io::AsRawHandle};
        use windows_sys::Win32::{Foundation::CloseHandle, System::JobObjects::*};
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() { return Err(std::io::Error::last_os_error().to_string()); }
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(handle, JobObjectExtendedLimitInformation, &limits as *const _ as _, size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32) == 0
                || AssignProcessToJobObject(handle, child.as_raw_handle()) == 0 {
                let error = std::io::Error::last_os_error().to_string();
                CloseHandle(handle);
                return Err(error);
            }
            Ok(Self(handle as usize))
        }
    }
}
#[cfg(windows)]
impl Drop for ProcessJob {
    fn drop(&mut self) { unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0 as _); } }
}

#[cfg(not(windows))]
pub struct ProcessJob;
#[cfg(not(windows))]
impl ProcessJob {
    pub fn process_ids(&self) -> Result<Vec<u32>,String> { Ok(vec![]) }
    pub fn describe_processes(&self, _ids: &[u32]) -> Vec<JobProcess> { vec![] }
    pub fn terminate_descendants(&self, _baseline: &[u32]) -> Result<Vec<u32>,String> { Ok(vec![]) }
    pub fn terminate_and_wait(&self) -> Result<(),String> { Ok(()) }
    pub fn assign(_child: &std::process::Child) -> Result<Self, String> { Err("首期运行时仅支持 Windows".into()) }
}
