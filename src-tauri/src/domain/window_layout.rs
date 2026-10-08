use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

pub const MIN_WINDOW_WIDTH: u32 = 800;
pub const MIN_WINDOW_HEIGHT: u32 = 600;
pub const MAX_LAYOUT_WINDOWS: usize = 32;

/// Physical desktop pixels, including the window frame. Never mix these with
/// WebView logical pixels; a monitor may have a negative desktop origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutMonitor {
    pub id: String,
    pub name: String,
    pub bounds: LayoutRect,
    pub work_area: LayoutRect,
    pub primary: bool,
    pub scale_factor: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutSlot {
    pub monitor_id: String,
    /// Relative to this monitor's work-area origin, in physical pixels.
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowLayout {
    pub id: String,
    pub name: String,
    pub monitors: Vec<LayoutMonitor>,
    /// Array order is launch order. Overlap is intentional and unrestricted.
    pub windows: Vec<LayoutSlot>,
}

impl WindowLayout {
    pub fn validate(&self) -> Result<(), AppError> {
        let invalid = |message: &str| AppError::ConfigWriteError(format!("窗口布局：{message}"));
        if self.id.trim().is_empty()
            || self.id.len() > 128
            || self.name.trim().is_empty()
            || self.name.chars().count() > 40
        {
            return Err(invalid("请输入有效的布局名称（最多 40 字）"));
        }
        if self.monitors.is_empty() || self.monitors.len() > 32 {
            return Err(invalid("显示器配置无效"));
        }
        let mut ids = HashSet::new();
        for monitor in &self.monitors {
            let work = monitor.work_area;
            let bounds = monitor.bounds;
            if monitor.id.is_empty()
                || !ids.insert(&monitor.id)
                || bounds.width == 0
                || bounds.height == 0
                || bounds.width > 32768
                || bounds.height > 32768
                || work.width == 0
                || work.height == 0
                || work.x < bounds.x
                || work.y < bounds.y
                || i64::from(work.x) + i64::from(work.width)
                    > i64::from(bounds.x) + i64::from(bounds.width)
                || i64::from(work.y) + i64::from(work.height)
                    > i64::from(bounds.y) + i64::from(bounds.height)
                || !monitor.scale_factor.is_finite()
                || !(0.5..=8.0).contains(&monitor.scale_factor)
            {
                return Err(invalid("显示器尺寸、工作区或缩放比例无效"));
            }
        }
        if self
            .monitors
            .iter()
            .filter(|monitor| monitor.primary)
            .count()
            != 1
        {
            return Err(invalid("必须有一个主显示器"));
        }
        if self.windows.is_empty() || self.windows.len() > MAX_LAYOUT_WINDOWS {
            return Err(invalid("窗口数量须在 1 到 32 之间"));
        }
        for slot in &self.windows {
            let monitor = self
                .monitors
                .iter()
                .find(|monitor| monitor.id == slot.monitor_id)
                .ok_or_else(|| invalid("窗口引用了不存在的显示器"))?;
            if slot.width < MIN_WINDOW_WIDTH || slot.height < MIN_WINDOW_HEIGHT {
                return Err(invalid("窗口最小尺寸为 800 × 600"));
            }
            if slot.x < 0
                || slot.y < 0
                || i64::from(slot.x) + i64::from(slot.width) > i64::from(monitor.work_area.width)
                || i64::from(slot.y) + i64::from(slot.height) > i64::from(monitor.work_area.height)
            {
                return Err(invalid("窗口必须完整位于显示器工作区内"));
            }
        }
        Ok(())
    }

    /// Preserve edge/center anchors when resolution changes. A disconnected
    /// monitor falls back to the primary; rectangles remain wholly reachable.
    pub fn resolve(&self, monitors: &[LayoutMonitor]) -> Result<Vec<LayoutRect>, AppError> {
        self.validate()?;
        let primary = monitors
            .iter()
            .find(|monitor| monitor.primary)
            .or_else(|| monitors.first())
            .ok_or_else(|| AppError::FileError("未检测到显示器".into()))?;
        self.windows
            .iter()
            .map(|slot| {
                let saved = self
                    .monitors
                    .iter()
                    .find(|monitor| monitor.id == slot.monitor_id)
                    .expect("validated monitor");
                let current = monitors
                    .iter()
                    .find(|monitor| monitor.id == slot.monitor_id)
                    .unwrap_or(primary);
                let work = current.work_area;
                if work.width < MIN_WINDOW_WIDTH || work.height < MIN_WINDOW_HEIGHT {
                    return Err(AppError::FileError(format!(
                        "{} 的工作区小于 800 × 600，无法应用布局",
                        current.name
                    )));
                }
                let width = slot.width.min(work.width);
                let height = slot.height.min(work.height);
                Ok(LayoutRect {
                    x: work.x.saturating_add(adapt_axis(
                        slot.x,
                        slot.width,
                        saved.work_area.width,
                        width,
                        work.width,
                    )),
                    y: work.y.saturating_add(adapt_axis(
                        slot.y,
                        slot.height,
                        saved.work_area.height,
                        height,
                        work.height,
                    )),
                    width,
                    height,
                })
            })
            .collect()
    }
}

fn adapt_axis(offset: i32, size: u32, previous: u32, next_size: u32, next: u32) -> i32 {
    let old_span = previous.saturating_sub(size);
    let anchor = if old_span == 0 {
        0.5
    } else {
        f64::from(offset) / f64::from(old_span)
    };
    (anchor.clamp(0.0, 1.0) * f64::from(next.saturating_sub(next_size))).round() as i32
}

pub fn validate_layout_configuration(
    layouts: &[WindowLayout],
    selected: Option<&str>,
) -> Result<(), AppError> {
    if layouts.len() > 64 {
        return Err(AppError::ConfigWriteError("最多保存 64 个窗口布局".into()));
    }
    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    for layout in layouts {
        layout.validate()?;
        if !ids.insert(&layout.id) || !names.insert(layout.name.trim().to_lowercase()) {
            return Err(AppError::ConfigWriteError(
                "窗口布局的名称和 ID 不能重复".into(),
            ));
        }
    }
    if selected.is_some_and(|id| !layouts.iter().any(|layout| layout.id == id)) {
        return Err(AppError::ConfigWriteError(
            "所选窗口布局不存在，请重新选择".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(id: &str, x: i32, width: u32, height: u32, primary: bool) -> LayoutMonitor {
        LayoutMonitor {
            id: id.into(),
            name: id.into(),
            bounds: LayoutRect {
                x,
                y: 0,
                width,
                height,
            },
            work_area: LayoutRect {
                x,
                y: 0,
                width,
                height,
            },
            primary,
            scale_factor: 1.0,
        }
    }

    fn layout() -> WindowLayout {
        WindowLayout {
            id: "desk".into(),
            name: "桌面".into(),
            monitors: vec![
                monitor("main", 0, 1920, 1040, true),
                monitor("left", -2560, 2560, 1400, false),
            ],
            windows: vec![LayoutSlot {
                monitor_id: "left".into(),
                x: 640,
                y: 340,
                width: 1280,
                height: 720,
            }],
        }
    }

    #[test]
    fn negative_desktop_origins_and_overlap_are_valid() {
        let mut value = layout();
        value.windows.push(value.windows[0].clone());
        let rectangles = value.resolve(&value.monitors).unwrap();
        assert_eq!(rectangles[0].x, -1920);
        assert_eq!(rectangles[0], rectangles[1]);
    }

    #[test]
    fn missing_monitor_and_smaller_work_area_keep_windows_inside() {
        let rectangles = layout()
            .resolve(&[monitor("main", 20, 1024, 768, true)])
            .unwrap();
        assert_eq!(
            rectangles[0],
            LayoutRect {
                x: 20,
                y: 24,
                width: 1024,
                height: 720
            }
        );
    }

    #[test]
    fn resolution_changes_preserve_center_and_right_edge() {
        let mut value = layout();
        value.windows.push(LayoutSlot {
            monitor_id: "left".into(),
            x: 1280,
            y: 680,
            width: 1280,
            height: 720,
        });
        let rectangles = value
            .resolve(&[monitor("left", -3840, 3840, 2120, true)])
            .unwrap();
        assert_eq!(rectangles[0].x, -2560);
        assert_eq!(rectangles[1].x, -1280);
        assert_eq!(rectangles[1].y, 1400);
    }

    #[test]
    fn invalid_sizes_offscreen_rectangles_and_missing_selection_are_rejected() {
        let mut value = layout();
        value.windows[0].width = 799;
        assert!(value.validate().is_err());
        value.windows[0].width = 1280;
        value.windows[0].x = 1281;
        assert!(value.validate().is_err());
        assert!(validate_layout_configuration(&[layout()], Some("missing")).is_err());
    }
}
