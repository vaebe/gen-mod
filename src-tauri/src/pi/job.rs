/// Job Object 生命周期与 pi 相同，关闭句柄清理包括命令工具在内的后代。
#[cfg(windows)]
pub struct ProcessJob(usize);
#[cfg(windows)]
impl ProcessJob {
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
    pub fn assign(_child: &std::process::Child) -> Result<Self, String> { Err("首期运行时仅支持 Windows".into()) }
}
