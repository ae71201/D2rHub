use crate::application::multi_instance::InstanceRegistry;
use crate::domain::{account::AccountMeta, config::GlobalConfig};
use crate::error::AppError;
use crate::launch_context::{ContextPurpose, LaunchContext};

pub(crate) fn commit(
    config: &GlobalConfig,
    registry: &InstanceRegistry,
    original: &AccountMeta,
    renamed: &AccountMeta,
    save: impl FnOnce() -> Result<(), AppError>,
) -> Result<(), AppError> {
    if original.display_name == renamed.display_name {
        return save();
    }
    registry.with_account_identity_change(&original.id, |registered_pid| {
        let context = LaunchContext::for_account(config, original, ContextPurpose::LaunchGame);
        // Offline accounts must remain editable before paths are configured.
        let executable = match context {
            Ok(context) => context.installation.game_executable,
            Err(_) if registered_pid.is_none() => return save(),
            Err(error) => return Err(error),
        };
        let old_title = if original.display_name.is_empty() {
            &original.id
        } else {
            &original.display_name
        };
        let pid = registered_pid.or_else(|| {
            super::system::find_unique_d2r_pid_by_window_identity(old_title, &executable)
        });
        let Some(pid) = pid else {
            return save();
        };
        let Some(window) = platform::VerifiedWindow::open(pid, old_title, &executable)? else {
            return save();
        };
        commit_with_rollback(
            old_title,
            &renamed.display_name,
            |title| window.set_title(title),
            save,
        )
    })
}

fn commit_with_rollback(
    old_title: &str,
    new_title: &str,
    mut set_title: impl FnMut(&str) -> Result<(), AppError>,
    save: impl FnOnce() -> Result<(), AppError>,
) -> Result<(), AppError> {
    let result = set_title(new_title).and_then(|()| save());
    if let Err(error) = result {
        return match set_title(old_title) {
            Ok(()) => Err(error),
            Err(rollback) => Err(AppError::Unknown(format!("账号改名失败：{error}；游戏窗口标题恢复失败：{rollback}。请退出该账号的游戏后重试。"))),
        };
    }
    Ok(())
}

#[cfg(target_os = "windows")]
mod platform {
    use super::AppError;
    use std::os::windows::{
        ffi::OsStringExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    };
    use std::path::{Path, PathBuf};
    use windows::{
        core::PWSTR,
        Win32::{
            Foundation::{HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
            System::Threading::{
                OpenProcess, QueryFullProcessImageNameW, WaitForSingleObject, PROCESS_NAME_WIN32,
                PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
            },
        },
    };

    extern "system" {
        fn EnumWindows(
            callback: unsafe extern "system" fn(isize, isize) -> i32,
            context: isize,
        ) -> i32;
        fn GetWindowThreadProcessId(hwnd: isize, pid: *mut u32) -> u32;
        fn IsWindowVisible(hwnd: isize) -> i32;
        fn GetWindowTextW(hwnd: isize, buffer: *mut u16, length: i32) -> i32;
        fn SendMessageTimeoutW(
            hwnd: isize,
            message: u32,
            wparam: usize,
            lparam: isize,
            flags: u32,
            timeout: u32,
            result: *mut usize,
        ) -> isize;
    }

    pub(super) struct VerifiedWindow {
        process: OwnedHandle,
        pid: u32,
        hwnd: isize,
    }

    impl VerifiedWindow {
        pub(super) fn open(
            pid: u32,
            title: &str,
            executable: &Path,
        ) -> Result<Option<Self>, AppError> {
            let raw = unsafe {
                OpenProcess(
                    PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                    false,
                    pid,
                )
            }
            .map_err(|error| {
                AppError::Unknown(format!("无法验证游戏进程，请刷新后重试：{error}"))
            })?;
            let process = unsafe { OwnedHandle::from_raw_handle(raw.0) };
            let handle = HANDLE(process.as_raw_handle());
            if unsafe { WaitForSingleObject(handle, 0) } == WAIT_OBJECT_0 {
                return Ok(None);
            }
            let mut buffer = vec![0u16; 32768];
            let mut length = buffer.len() as u32;
            unsafe {
                QueryFullProcessImageNameW(
                    handle,
                    PROCESS_NAME_WIN32,
                    PWSTR(buffer.as_mut_ptr()),
                    &mut length,
                )
            }
            .map_err(|error| AppError::Unknown(error.to_string()))?;
            let actual = PathBuf::from(std::ffi::OsString::from_wide(&buffer[..length as usize]));
            if !crate::launch_context::paths_have_same_identity(&actual, executable)
                || crate::infrastructure::system::find_unique_d2r_pid_by_window_identity(
                    title, executable,
                ) != Some(pid)
            {
                return Err(AppError::Unknown("游戏进程身份不匹配，未执行改名".into()));
            }
            struct Search<'a> {
                pid: u32,
                title: &'a str,
                hwnd: isize,
            }
            unsafe extern "system" fn find(hwnd: isize, context: isize) -> i32 {
                let search = &mut *(context as *mut Search<'_>);
                let mut pid = 0;
                GetWindowThreadProcessId(hwnd, &mut pid);
                if pid == search.pid
                    && IsWindowVisible(hwnd) != 0
                    && read_title(hwnd).eq_ignore_ascii_case(search.title)
                {
                    search.hwnd = hwnd;
                    return 0;
                }
                1
            }
            let mut search = Search {
                pid,
                title,
                hwnd: 0,
            };
            unsafe {
                EnumWindows(find, &mut search as *mut Search<'_> as isize);
            }
            if search.hwnd == 0 {
                return Err(AppError::Unknown(
                    "未找到账号的游戏窗口，请刷新后重试".into(),
                ));
            }
            Ok(Some(Self {
                process,
                pid,
                hwnd: search.hwnd,
            }))
        }

        pub(super) fn set_title(&self, title: &str) -> Result<(), AppError> {
            let status = unsafe { WaitForSingleObject(HANDLE(self.process.as_raw_handle()), 0) };
            if status == WAIT_OBJECT_0 {
                return Ok(());
            }
            let mut pid = 0;
            unsafe {
                GetWindowThreadProcessId(self.hwnd, &mut pid);
            }
            if status != WAIT_TIMEOUT || pid != self.pid {
                return Err(AppError::Unknown("游戏窗口身份发生变化，未执行改名".into()));
            }
            let text: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
            let mut result = 0;
            // Bound cross-process WM_SETTEXT delivery just like other window commands.
            let sent = unsafe {
                SendMessageTimeoutW(
                    self.hwnd,
                    0x000C,
                    0,
                    text.as_ptr() as isize,
                    0x0001 | 0x0002 | 0x0020,
                    1000,
                    &mut result,
                )
            };
            if sent == 0 || result == 0 || read_title(self.hwnd) != title {
                return Err(AppError::Unknown("游戏窗口标题同步失败或超时".into()));
            }
            Ok(())
        }
    }

    fn read_title(hwnd: isize) -> String {
        let mut buffer = [0u16; 512];
        let length = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
        String::from_utf16_lossy(&buffer[..length.max(0) as usize])
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    use super::AppError;
    pub(super) struct VerifiedWindow;
    impl VerifiedWindow {
        pub(super) fn open(_: u32, _: &str, _: &std::path::Path) -> Result<Option<Self>, AppError> {
            Err(AppError::Unknown("仅支持 Windows".into()))
        }
        pub(super) fn set_title(&self, _: &str) -> Result<(), AppError> {
            Err(AppError::Unknown("仅支持 Windows".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{commit_with_rollback, AppError};
    use std::cell::RefCell;

    #[test]
    fn rename_verifies_the_window_before_saving_and_does_not_restore_on_success() {
        let events = RefCell::new(Vec::new());
        commit_with_rollback(
            "Old",
            "New",
            |title| {
                events.borrow_mut().push(title.to_string());
                Ok(())
            },
            || {
                events.borrow_mut().push("save".into());
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(*events.borrow(), ["New", "save"]);
    }

    #[test]
    fn failed_window_sync_never_saves_and_restores_the_original_title() {
        let events = RefCell::new(Vec::new());
        let result = commit_with_rollback(
            "Old",
            "New",
            |title| {
                events.borrow_mut().push(title.to_string());
                if title == "New" {
                    Err(AppError::Unknown("window denied".into()))
                } else {
                    Ok(())
                }
            },
            || panic!("Must not save an unverified name"),
        );
        assert!(result.unwrap_err().to_string().contains("window denied"));
        assert_eq!(*events.borrow(), ["New", "Old"]);
    }

    #[test]
    fn failed_persistence_restores_the_title_and_reports_rollback_failure() {
        for rollback_fails in [false, true] {
            let events = RefCell::new(Vec::new());
            let result = commit_with_rollback(
                "Old",
                "New",
                |title| {
                    events.borrow_mut().push(title.to_string());
                    if title == "Old" && rollback_fails {
                        Err(AppError::Unknown("rollback denied".into()))
                    } else {
                        Ok(())
                    }
                },
                || Err(AppError::FileError("disk denied".into())),
            );
            let error = result.unwrap_err().to_string();
            assert!(error.contains("disk denied"));
            assert_eq!(error.contains("rollback denied"), rollback_fails);
            assert_eq!(*events.borrow(), ["New", "Old"]);
        }
    }
}
