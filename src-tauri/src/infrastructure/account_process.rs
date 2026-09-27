// Keep a handle to the original process throughout validation, termination and
// exit verification. Never taskkill a bare PID or fall back to killing by name.
#[cfg(target_os = "windows")]
pub(crate) fn terminate_verified(
    pid: u32,
    title: &str,
    executable: &std::path::Path,
) -> Result<(), String> {
    use std::os::windows::{
        ffi::OsStringExt,
        io::{AsRawHandle, FromRawHandle},
    };
    use windows::{
        core::PWSTR,
        Win32::{
            Foundation::{HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
            System::Threading::{
                OpenProcess, QueryFullProcessImageNameW, TerminateProcess, WaitForSingleObject,
                PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
                PROCESS_TERMINATE,
            },
        },
    };
    let raw = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE | PROCESS_TERMINATE,
            false,
            pid,
        )
    }
    .map_err(|e| e.to_string())?;
    let owned = unsafe { std::os::windows::io::OwnedHandle::from_raw_handle(raw.0) };
    let handle = HANDLE(owned.as_raw_handle());
    if unsafe { WaitForSingleObject(handle, 0) } == WAIT_OBJECT_0 {
        return Ok(());
    }
    let mut image = vec![0u16; 32768];
    let mut length = image.len() as u32;
    unsafe {
        QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(image.as_mut_ptr()),
            &mut length,
        )
    }
    .map_err(|e| e.to_string())?;
    let actual = std::path::PathBuf::from(std::ffi::OsString::from_wide(&image[..length as usize]));
    if !matches_close_target(
        pid,
        crate::infrastructure::system::find_unique_d2r_pid_by_window_identity(title, executable),
        &actual,
        executable,
    ) {
        return Err("游戏进程身份不匹配，未执行关闭".into());
    }
    let status = unsafe { WaitForSingleObject(handle, 0) };
    if status == WAIT_OBJECT_0 {
        return Ok(());
    }
    if status != WAIT_TIMEOUT {
        return Err("无法确认目标进程状态，未执行关闭".into());
    }
    unsafe { TerminateProcess(handle, 1) }.map_err(|e| e.to_string())?;
    if unsafe { WaitForSingleObject(handle, 3000) } != WAIT_OBJECT_0 {
        return Err("游戏进程尚未退出，请稍后重试".into());
    }
    Ok(())
}

#[cfg(any(target_os = "windows", test))]
fn matches_close_target(
    pid: u32,
    unique_pid: Option<u32>,
    actual: &std::path::Path,
    expected: &std::path::Path,
) -> bool {
    pid != 0
        && unique_pid == Some(pid)
        && crate::launch_context::paths_have_same_identity(actual, expected)
}

#[cfg(test)]
mod tests {
    use super::matches_close_target;

    #[test]
    fn close_requires_both_the_exact_executable_and_unique_account_pid() {
        let executable = std::env::current_exe().unwrap();
        assert!(matches_close_target(42, Some(42), &executable, &executable));
        assert!(!matches_close_target(
            42,
            Some(43),
            &executable,
            &executable
        ));
        assert!(!matches_close_target(42, None, &executable, &executable));
        assert!(!matches_close_target(
            42,
            Some(42),
            &executable,
            &executable.with_file_name("wrong-edition.exe")
        ));
        assert!(!matches_close_target(0, Some(0), &executable, &executable));
    }
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn terminate_verified(_: u32, _: &str, _: &std::path::Path) -> Result<(), String> {
    Err("仅支持 Windows".into())
}
