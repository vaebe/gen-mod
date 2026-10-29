use std::path::Path;

pub fn copy_to_staging(from: &Path, staged: &Path) -> Result<String, String> {
    use std::io::Write;
    validate_plain_file(from)?;
    let mut input = std::fs::File::open(from).map_err(|e| e.to_string())?;
    let before = input.metadata().map_err(|e| e.to_string())?;
    let mut output = std::fs::OpenOptions::new().write(true).create_new(true)
        .open(staged).map_err(|e| e.to_string())?;
    let copied = std::io::copy(&mut input, &mut output).map_err(|e| e.to_string())?;
    let after = input.metadata().map_err(|e| e.to_string())?;
    if copied != before.len() || before.len() != after.len()
        || before.modified().map_err(|e| e.to_string())? != after.modified().map_err(|e| e.to_string())? {
        return Err("导入期间源文件发生变化，请重试".into());
    }
    output.flush().map_err(|e| e.to_string())?;
    output.sync_all().map_err(|e| e.to_string())?;
    drop(output);
    crate::changes::file_hash(staged)
}

pub fn validate_plain_file(path: &Path) -> Result<(), String> {
    for ancestor in path.ancestors() { crate::storage::paths::reject_redirect(ancestor)?; }
    if !std::fs::symlink_metadata(path).map_err(|e| e.to_string())?.is_file() {
        return Err("只能操作普通文件".into());
    }
    Ok(())
}

pub fn move_no_replace(from: &Path, to: &Path) -> Result<(), String> {
    validate_plain_file(from)?;
    move_entry_no_replace(from, to)
}

pub fn move_entry_no_replace(from: &Path, to: &Path) -> Result<(), String> {
    for ancestor in from.ancestors() { crate::storage::paths::reject_redirect(ancestor)?; }
    let metadata = std::fs::symlink_metadata(from).map_err(|e| e.to_string())?;
    if !metadata.is_file() && !metadata.is_dir() { return Err("只能操作普通文件或文件夹".into()); }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let from = from.canonicalize().map_err(|e| e.to_string())?;
        let parent = to.parent().ok_or("目标缺少父目录")?.canonicalize().map_err(|e| e.to_string())?;
        let to = parent.join(to.file_name().ok_or("目标缺少文件名")?);
        let from: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
        // MoveFileW 不替换已有目标，避免检查存在与 rename 之间的竞争覆盖。
        if unsafe { windows_sys::Win32::Storage::FileSystem::MoveFileW(from.as_ptr(), to.as_ptr()) } == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = to;
        Err("当前文件移动只支持 Windows".into())
    }
}
