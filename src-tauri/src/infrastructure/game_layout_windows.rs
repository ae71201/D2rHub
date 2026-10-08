//! On-demand physical monitor/window snapshots. No process polling or global
//! hooks are needed for layout editing, capture, or restore.

use std::path::PathBuf;

use crate::domain::window_layout::{LayoutMonitor, LayoutRect};
use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct GameLayoutWindow {
    pub handle: isize,
    pub pid: u32,
    pub title: String,
    pub executable: PathBuf,
    pub started_at: u64,
    pub rect: LayoutRect,
    pub minimized: bool,
}

#[cfg(target_os = "windows")]
mod native {
    use std::collections::HashMap;
    use std::os::windows::ffi::OsStringExt;

    use windows::core::PWSTR;
    use windows::Win32::Foundation::{CloseHandle, BOOL, FILETIME, HWND, LPARAM, RECT};
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
    };
    use windows::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::HiDpi::{
        GetDpiForMonitor, SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT,
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, MDT_EFFECTIVE_DPI,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindow, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
        IsWindow, IsWindowVisible, IsZoomed, SetWindowPos, ShowWindowAsync, GW_OWNER, HWND_TOP,
        MONITORINFOF_PRIMARY, SWP_ASYNCWINDOWPOS, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SWP_NOSIZE,
        SWP_NOZORDER, SW_RESTORE,
    };

    use super::*;

    pub(crate) struct PhysicalDpiScope(DPI_AWARENESS_CONTEXT);

    impl PhysicalDpiScope {
        pub(crate) fn enter() -> Self {
            Self(unsafe {
                SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)
            })
        }
    }

    impl Drop for PhysicalDpiScope {
        fn drop(&mut self) {
            if !self.0 .0.is_null() {
                unsafe {
                    SetThreadDpiAwarenessContext(self.0);
                }
            }
        }
    }

    fn rectangle(rect: RECT) -> LayoutRect {
        LayoutRect {
            x: rect.left,
            y: rect.top,
            width: rect.right.saturating_sub(rect.left).max(0) as u32,
            height: rect.bottom.saturating_sub(rect.top).max(0) as u32,
        }
    }

    pub fn monitors() -> Result<Vec<LayoutMonitor>, AppError> {
        let _dpi = PhysicalDpiScope::enter();
        unsafe extern "system" fn collect(
            monitor: HMONITOR,
            _: HDC,
            _: *mut RECT,
            data: LPARAM,
        ) -> BOOL {
            let result = &mut *(data.0 as *mut Vec<LayoutMonitor>);
            let mut info = MONITORINFOEXW::default();
            info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
            if GetMonitorInfoW(monitor, &mut info.monitorInfo).as_bool() {
                let end = info
                    .szDevice
                    .iter()
                    .position(|&unit| unit == 0)
                    .unwrap_or(info.szDevice.len());
                let id = String::from_utf16_lossy(&info.szDevice[..end]);
                let (mut dpi_x, mut dpi_y) = (96, 96);
                let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
                result.push(LayoutMonitor {
                    name: id.trim_start_matches(r"\\.\").to_string(),
                    id,
                    bounds: rectangle(info.monitorInfo.rcMonitor),
                    work_area: rectangle(info.monitorInfo.rcWork),
                    primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
                    scale_factor: f64::from(dpi_x) / 96.0,
                });
            }
            BOOL(1)
        }
        let mut result: Vec<LayoutMonitor> = Vec::new();
        unsafe {
            if !EnumDisplayMonitors(
                None,
                None,
                Some(collect),
                LPARAM(&mut result as *mut _ as isize),
            )
            .as_bool()
            {
                return Err(AppError::FileError("读取显示器失败".into()));
            }
        }
        result.sort_by_key(|monitor| (!monitor.primary, monitor.bounds.x, monitor.bounds.y));
        if result.is_empty() {
            return Err(AppError::FileError("未检测到显示器".into()));
        }
        Ok(result)
    }

    fn process_identity(pid: u32) -> Option<(PathBuf, u64)> {
        unsafe {
            let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let read = (|| {
                let mut path = vec![0u16; 32768];
                let mut length = path.len() as u32;
                QueryFullProcessImageNameW(
                    process,
                    PROCESS_NAME_WIN32,
                    PWSTR(path.as_mut_ptr()),
                    &mut length,
                )
                .ok()?;
                let executable =
                    PathBuf::from(std::ffi::OsString::from_wide(&path[..length as usize]));
                if !executable
                    .file_name()?
                    .to_string_lossy()
                    .eq_ignore_ascii_case("D2R.exe")
                {
                    return None;
                }
                let (mut creation, mut exit, mut kernel, mut user) = (
                    FILETIME::default(),
                    FILETIME::default(),
                    FILETIME::default(),
                    FILETIME::default(),
                );
                GetProcessTimes(process, &mut creation, &mut exit, &mut kernel, &mut user).ok()?;
                Some((
                    executable,
                    (u64::from(creation.dwHighDateTime) << 32) | u64::from(creation.dwLowDateTime),
                ))
            })();
            let _ = CloseHandle(process);
            read
        }
    }

    pub fn snapshot() -> Result<Vec<GameLayoutWindow>, AppError> {
        let _dpi = PhysicalDpiScope::enter();
        #[derive(Default)]
        struct Scan {
            processes: HashMap<u32, Option<(PathBuf, u64)>>,
            windows: HashMap<u32, GameLayoutWindow>,
        }
        unsafe extern "system" fn collect(hwnd: HWND, data: LPARAM) -> BOOL {
            if !IsWindowVisible(hwnd).as_bool() {
                return BOOL(1);
            }
            if GetWindow(hwnd, GW_OWNER).is_ok_and(|owner| !owner.0.is_null()) {
                return BOOL(1);
            }
            let scan = &mut *(data.0 as *mut Scan);
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            let Some((executable, started_at)) = scan
                .processes
                .entry(pid)
                .or_insert_with(|| process_identity(pid))
                .clone()
            else {
                return BOOL(1);
            };
            let mut title = [0u16; 512];
            let length = GetWindowTextW(hwnd, &mut title);
            if length <= 0 {
                return BOOL(1);
            }
            let mut rect = RECT::default();
            if GetWindowRect(hwnd, &mut rect).is_err() {
                return BOOL(1);
            }
            let rect = rectangle(rect);
            let window = GameLayoutWindow {
                handle: hwnd.0 as isize,
                pid,
                title: String::from_utf16_lossy(&title[..length as usize]),
                executable,
                started_at,
                rect,
                minimized: IsIconic(hwnd).as_bool(),
            };
            // Game-owned dialogs must not replace the largest/main window.
            if scan.windows.get(&pid).is_none_or(|previous| {
                u64::from(previous.rect.width) * u64::from(previous.rect.height)
                    < u64::from(rect.width) * u64::from(rect.height)
            }) {
                scan.windows.insert(pid, window);
            }
            BOOL(1)
        }
        let mut scan = Scan::default();
        unsafe { EnumWindows(Some(collect), LPARAM(&mut scan as *mut _ as isize)) }
            .map_err(|error| AppError::FileError(format!("读取游戏窗口失败：{error}")))?;
        let mut windows: Vec<_> = scan.windows.into_values().collect();
        windows.sort_by_key(|window| (window.started_at, window.pid));
        Ok(windows)
    }

    fn apply_handle(hwnd: HWND, rect: LayoutRect, raise: bool) -> Result<(), AppError> {
        let _dpi = PhysicalDpiScope::enter();
        unsafe {
            if IsIconic(hwnd).as_bool() || IsZoomed(hwnd).as_bool() {
                let _ = ShowWindowAsync(hwnd, SW_RESTORE);
            }
            let flags = SWP_NOACTIVATE
                | SWP_NOOWNERZORDER
                | SWP_ASYNCWINDOWPOS
                | SWP_NOSIZE
                | if raise {
                    Default::default()
                } else {
                    SWP_NOZORDER
                };
            SetWindowPos(hwnd, HWND_TOP, rect.x, rect.y, 0, 0, flags)
                .map_err(|error| AppError::FileError(format!("调整游戏窗口失败：{error}")))
        }
    }

    pub fn apply(window: &GameLayoutWindow, rect: LayoutRect, raise: bool) -> Result<(), AppError> {
        let hwnd = HWND(window.handle as *mut _);
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if !IsWindow(hwnd).as_bool() || pid != window.pid {
                return Err(AppError::FileError("游戏窗口已关闭，请重试".into()));
            }
        }
        if process_identity(pid).is_none_or(|(path, started)| {
            started != window.started_at
                || !crate::infrastructure::system::executable_paths_match(&path, &window.executable)
        }) {
            return Err(AppError::FileError("游戏进程已变化，请重试".into()));
        }
        apply_handle(hwnd, rect, raise)
    }

    pub fn apply_for_pid(pid: u32, executable: &std::path::Path, rect: LayoutRect) -> bool {
        if process_identity(pid).is_none_or(|(path, _)| {
            !crate::infrastructure::system::executable_paths_match(&path, executable)
        }) {
            return false;
        }
        crate::infrastructure::system::find_game_hwnd(pid)
            .is_some_and(|handle| apply_handle(HWND(handle as *mut _), rect, false).is_ok())
    }
}

#[cfg(target_os = "windows")]
pub(crate) use native::PhysicalDpiScope;
#[cfg(target_os = "windows")]
pub use native::{apply, apply_for_pid, monitors, snapshot};

#[cfg(not(target_os = "windows"))]
pub fn monitors() -> Result<Vec<LayoutMonitor>, AppError> {
    Err(AppError::FileError("窗口布局仅支持 Windows".into()))
}
#[cfg(not(target_os = "windows"))]
pub fn snapshot() -> Result<Vec<GameLayoutWindow>, AppError> {
    Err(AppError::FileError("窗口布局仅支持 Windows".into()))
}
#[cfg(not(target_os = "windows"))]
pub fn apply(_: &GameLayoutWindow, _: LayoutRect, _: bool) -> Result<(), AppError> {
    Err(AppError::FileError("窗口布局仅支持 Windows".into()))
}
#[cfg(not(target_os = "windows"))]
pub fn apply_for_pid(_: u32, _: &std::path::Path, _: LayoutRect) -> bool {
    false
}
