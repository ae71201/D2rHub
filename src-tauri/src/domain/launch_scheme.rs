//! Saved launch recipes own values, never live references to mutable preferences.
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{
    config::{GlobalConfig, LaunchGroup},
    window_layout::WindowLayout,
};
use crate::{domain::room_automation::RoomAutomationConfig, error::AppError};

pub const RUNTIME_LAYOUT_ID: &str = "__launch_session_layout__";
pub const SETTING_KEYS: &[&str] = &[
    "rune_audio_enabled",
    "rune_audio_target_account",
    "rune_audio_external_target",
    "rune_audio_detection_threshold",
    "rune_audio_tracked_categories",
    "rune_audio_min_rune_number",
    "rune_audio_min_gem_level",
    "rune_audio_tracked_charm_codes",
    "enable_tz_overlay",
    "enable_stats_overlay",
    "launch_trim_memory",
    "keep_battle_net_account_id",
    "auto_close_browser",
    "agent_mode",
    "agent_delay_secs",
    "agent_threshold",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemeEnvironment {
    pub settings: Map<String, Value>,
    pub room: RoomAutomationConfig,
    pub layout: Option<WindowLayout>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountLaunchSnapshot {
    pub game_settings: Map<String, Value>,
    pub position: Option<(i32, i32)>,
    pub region: Option<String>,
    pub language: Option<String>,
    pub voicelanguage: Option<String>,
}

/// Provenance and immutable retry input. Current module settings remain in their
/// existing durable stores; they never write back into this recipe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaunchSession {
    pub id: String,
    pub scheme: LaunchGroup,
    pub started_at: String,
    #[serde(default)]
    pub processes: Vec<SessionProcess>,
    /// Working account settings. The recipe above is immutable provenance.
    #[serde(default)]
    pub members: Vec<super::config::LaunchGroupMember>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionProcess {
    pub account_id: String,
    pub pid: u32,
    pub started_at: u64,
    pub executable: std::path::PathBuf,
    pub mod_args: String,
}

pub fn settings_snapshot(config: &GlobalConfig) -> Map<String, Value> {
    let all = serde_json::to_value(config).expect("serializable configuration");
    SETTING_KEYS
        .iter()
        .map(|key| ((*key).into(), all[*key].clone()))
        .collect()
}

/// The working copy lives beside library templates for compatibility with the
/// layout UI. Give only that copy a distinct display name, using the same
/// whitespace/case rules and 40-character bound as layout validation.
fn runtime_layout_name(source: &str, library: &[WindowLayout]) -> String {
    let occupied: std::collections::HashSet<_> = library
        .iter()
        .map(|layout| layout.name.trim().to_lowercase())
        .collect();
    for index in 1..=library.len() + 1 {
        let suffix = if index == 1 {
            "（本次运行）".to_string()
        } else {
            format!("（本次运行 {index}）")
        };
        let base: String = source
            .trim()
            .chars()
            .take(40 - suffix.chars().count())
            .collect();
        let candidate = format!("{}{suffix}", base.trim_end());
        if !occupied.contains(&candidate.to_lowercase()) {
            return candidate;
        }
    }
    unreachable!("a finite layout library cannot occupy every generated name")
}

impl SchemeEnvironment {
    pub fn project(&self, current: &GlobalConfig) -> Result<GlobalConfig, AppError> {
        if self.settings.len() != SETTING_KEYS.len()
            || self
                .settings
                .keys()
                .any(|key| !SETTING_KEYS.contains(&key.as_str()))
        {
            return Err(AppError::ConfigReadError(
                "方案设置不完整，请编辑并保存方案".into(),
            ));
        }
        let mut value = serde_json::to_value(current)?;
        value.as_object_mut().unwrap().extend(self.settings.clone());
        let mut config: GlobalConfig = serde_json::from_value(value)?;
        config
            .window_layouts
            .retain(|layout| layout.id != RUNTIME_LAYOUT_ID);
        config.window_layout_enabled = self.layout.is_some();
        config.active_window_layout_id = self.layout.as_ref().map(|_| RUNTIME_LAYOUT_ID.into());
        if let Some(mut layout) = self.layout.clone() {
            // Do not hide invalid saved names by truncating them during projection.
            layout.validate()?;
            layout.id = RUNTIME_LAYOUT_ID.into();
            layout.name = runtime_layout_name(&layout.name, &config.window_layouts);
            config.window_layouts.push(layout);
        }
        // Validate the complete projected state before any module settings are
        // committed, not only the standalone layout held in the recipe.
        super::window_layout::validate_layout_configuration(
            &config.window_layouts,
            config.active_window_layout_id.as_deref(),
        )?;
        Ok(config)
    }

    pub fn validate(&self, group: &LaunchGroup, current: &GlobalConfig) -> Result<(), AppError> {
        let config = self.project(current)?;
        let contains = |id: &str| group.members.iter().any(|m| m.account_id == id);
        if config.rune_audio_enabled
            && (config.rune_audio_external_target.is_some()
                || !contains(&config.rune_audio_target_account))
        {
            return Err(AppError::ConfigReadError("请选择方案内的识别账号".into()));
        }
        if config
            .keep_battle_net_account_id
            .as_deref()
            .is_some_and(|id| !contains(id))
        {
            return Err(AppError::ConfigReadError(
                "保留战网的账号必须属于本方案".into(),
            ));
        }
        if self.room.enabled {
            if (!self.room.primary_account_id.is_empty()
                && !contains(&self.room.primary_account_id))
                || self
                    .room
                    .follower_account_ids
                    .iter()
                    .any(|id| !contains(id))
            {
                return Err(AppError::ConfigReadError(
                    "跟房主号和小号必须属于本方案".into(),
                ));
            }
            self.room
                .validate(std::iter::empty::<&str>())
                .map_err(|error| AppError::ConfigReadError(error.to_string()))?;
        }
        if let Some(layout) = &self.layout {
            super::window_layout::validate_layout_configuration(
                std::slice::from_ref(layout),
                Some(&layout.id),
            )?;
            if layout.windows.len() < group.members.len() {
                return Err(AppError::ConfigReadError(
                    "布局位置少于方案账号，请补充位置".into(),
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::window_layout::{
        validate_layout_configuration, LayoutMonitor, LayoutRect, LayoutSlot,
    };
    use serde_json::json;

    fn layout(id: &str, name: &str) -> WindowLayout {
        let bounds = LayoutRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        WindowLayout {
            id: id.into(),
            name: name.into(),
            monitors: vec![LayoutMonitor {
                id: "display".into(),
                name: "Display".into(),
                bounds,
                work_area: bounds,
                primary: true,
                scale_factor: 1.0,
                frame: None,
            }],
            windows: vec![LayoutSlot {
                monitor_id: "display".into(),
                x: 0,
                y: 0,
                width: 1280,
                height: 720,
            }],
        }
    }

    #[test]
    fn launching_a_copied_library_layout_keeps_both_layouts_valid_and_independent() {
        let template = layout("desk", "日常多开");
        let config = GlobalConfig {
            window_layouts: vec![template.clone()],
            ..Default::default()
        };
        let mut recipe = environment(&config);
        recipe.layout = Some(template.clone());
        let mut projected = recipe.project(&config).unwrap();
        validate_layout_configuration(
            &projected.window_layouts,
            projected.active_window_layout_id.as_deref(),
        )
        .unwrap();
        assert_eq!(projected.window_layouts[0], template);
        assert_eq!(
            projected.active_window_layout_id.as_deref(),
            Some(RUNTIME_LAYOUT_ID)
        );
        assert_ne!(projected.window_layouts[1].name, template.name);
        projected.window_layouts[1].windows[0].x = 100;
        assert_eq!(recipe.layout, Some(template.clone()));
        assert_eq!(
            config.window_layouts.as_slice(),
            std::slice::from_ref(&template)
        );

        let relaunched = recipe.project(&projected).unwrap();
        assert_eq!(relaunched.window_layouts.len(), 2);
        assert_eq!(relaunched.window_layouts[1].windows, template.windows);
        validate_layout_configuration(
            &relaunched.window_layouts,
            relaunched.active_window_layout_id.as_deref(),
        )
        .unwrap();
    }

    #[test]
    fn runtime_name_handles_case_whitespace_and_existing_numbered_names() {
        let config = GlobalConfig {
            window_layouts: vec![
                layout("one", " DESK（本次运行） "),
                layout("two", "Desk（本次运行 2）"),
            ],
            ..Default::default()
        };
        let mut recipe = environment(&config);
        recipe.layout = Some(layout("source", "desk"));
        let first = recipe.project(&config).unwrap();
        assert_eq!(
            first.window_layouts.last().unwrap().name,
            "desk（本次运行 3）"
        );
        let second = recipe.project(&first).unwrap();
        assert_eq!(first.window_layouts, second.window_layouts);
        recipe.layout = None;
        let disabled = recipe.project(&second).unwrap();
        assert_eq!(disabled.window_layouts, config.window_layouts);
        assert!(!disabled.window_layout_enabled);
        assert!(disabled.active_window_layout_id.is_none());
    }

    #[test]
    fn runtime_names_respect_unicode_length_without_changing_the_saved_name() {
        let name = "窗".repeat(40);
        let template = layout("long", &name);
        let config = GlobalConfig {
            window_layouts: vec![template.clone()],
            ..Default::default()
        };
        let mut recipe = environment(&config);
        recipe.layout = Some(template);
        let first = recipe.project(&config).unwrap();
        let mut collision = first.window_layouts.last().unwrap().clone();
        assert_eq!(collision.name.chars().count(), 40);
        collision.id = "another-template".into();
        let mut occupied = config.clone();
        occupied.window_layouts.push(collision);
        let projected = recipe.project(&occupied).unwrap();
        assert_eq!(
            projected
                .window_layouts
                .last()
                .unwrap()
                .name
                .chars()
                .count(),
            40
        );
        assert_eq!(recipe.layout.as_ref().unwrap().name, name);
        validate_layout_configuration(
            &projected.window_layouts,
            projected.active_window_layout_id.as_deref(),
        )
        .unwrap();
    }

    #[test]
    fn projection_rejects_invalid_library_before_any_settings_can_be_applied() {
        let config = GlobalConfig {
            window_layouts: vec![layout("one", "duplicate"), layout("two", " DUPLICATE ")],
            ..Default::default()
        };
        assert!(environment(&config).project(&config).is_err());
        let valid = GlobalConfig::default();
        let mut recipe = environment(&valid);
        recipe.layout = Some(layout("source", &"窗".repeat(41)));
        assert!(recipe.project(&valid).is_err());
    }

    fn environment(config: &GlobalConfig) -> SchemeEnvironment {
        SchemeEnvironment {
            settings: settings_snapshot(config),
            room: Default::default(),
            layout: None,
        }
    }

    #[test]
    fn repeated_launch_uses_recipe_values_not_last_runtime_changes() {
        let defaults = GlobalConfig {
            rune_audio_target_account: "a".into(),
            ..Default::default()
        };
        let recipe = environment(&defaults);
        let mut runtime = recipe.project(&defaults).unwrap();
        runtime.rune_audio_target_account = "b".into();
        runtime.rune_audio_tracked_categories.clear();
        let next_launch = recipe.project(&runtime).unwrap();
        assert_eq!(next_launch.rune_audio_target_account, "a");
        assert_eq!(
            next_launch.rune_audio_tracked_categories,
            defaults.rune_audio_tracked_categories
        );
        assert_eq!(recipe.settings["rune_audio_target_account"], "a");
    }

    #[test]
    fn projection_preserves_public_resources_and_other_schemes() {
        let mut config = GlobalConfig {
            theme: "forest".into(),
            accounts_dir: "accounts".into(),
            ..Default::default()
        };
        config.launch_groups.push(LaunchGroup {
            id: "other".into(),
            name: "Other".into(),
            ..Default::default()
        });
        let projected = environment(&GlobalConfig::default())
            .project(&config)
            .unwrap();
        assert_eq!(projected.theme, config.theme);
        assert_eq!(projected.accounts_dir, config.accounts_dir);
        assert_eq!(projected.launch_groups, config.launch_groups);
    }

    #[test]
    fn missing_or_injected_fields_fail_closed() {
        let config = GlobalConfig::default();
        let mut recipe = environment(&config);
        recipe.settings.remove("rune_audio_enabled");
        assert!(recipe.project(&config).is_err());
        recipe
            .settings
            .insert("accounts_dir".into(), json!("unexpected"));
        assert!(recipe.project(&config).is_err());
    }

    #[test]
    fn recognition_and_room_targets_must_belong_to_recipe() {
        let config = GlobalConfig::default();
        let mut recipe = environment(&config);
        let scheme = LaunchGroup::default();
        recipe
            .settings
            .insert("rune_audio_enabled".into(), json!(true));
        recipe
            .settings
            .insert("rune_audio_target_account".into(), json!("outside"));
        assert!(recipe.validate(&scheme, &config).is_err());
        recipe
            .settings
            .insert("rune_audio_enabled".into(), json!(false));
        recipe.room.enabled = true;
        recipe.room.primary_account_id = "outside".into();
        assert!(recipe.validate(&scheme, &config).is_err());
    }
}
