use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::domain::account::AccountMeta;
use crate::error::AppError;

use super::LaunchOrchestrator;

/// Per-account values that exist only for one launch request.
///
/// The application model deliberately owns their validation so neither the
/// React caller nor the Tauri adapter can bypass the core launch invariants.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LaunchAccountOverrides {
    #[serde(default)]
    pub snapshot: Option<crate::domain::launch_scheme::AccountLaunchSnapshot>,
    pub mod_args: String,
    #[serde(default)]
    pub position_preset_id: Option<String>,
    #[serde(default)]
    pub resolution: Option<String>,
    #[serde(default)]
    pub fps: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LaunchAccountEntry {
    pub account_id: String,
    pub overrides: LaunchAccountOverrides,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchGraphicsOverride {
    pub game_settings: Option<serde_json::Map<String, serde_json::Value>>,
    pub resolution: String,
    pub fps: u32,
}

/// Validated shape of either a default-account launch or a launch-plan run.
///
/// An empty request remains a valid no-op for IPC compatibility. A plan can
/// never contain duplicate account identities or silently lose an override to
/// a `HashMap` replacement.
#[derive(Debug, Clone, Default)]
pub struct LaunchBatchPlan {
    account_ids: Vec<String>,
    overrides_by_account: HashMap<String, LaunchAccountOverrides>,
}

impl LaunchBatchPlan {
    pub fn from_request(
        account_ids: Option<Vec<String>>,
        entries: Option<Vec<LaunchAccountEntry>>,
    ) -> Result<Self, AppError> {
        let (account_ids, overrides_by_account) = match (account_ids, entries) {
            (Some(_), Some(_)) => {
                return Err(AppError::ConfigReadError(
                    "启动请求不能同时包含默认账号列表和方案配置".to_string(),
                ));
            }
            (Some(account_ids), None) => (account_ids, HashMap::new()),
            (None, Some(entries)) => {
                let account_ids = entries
                    .iter()
                    .map(|entry| entry.account_id.clone())
                    .collect::<Vec<_>>();
                LaunchOrchestrator::validate_account_ids(&account_ids)?;
                let overrides_by_account = entries
                    .into_iter()
                    .map(|entry| (entry.account_id, entry.overrides))
                    .collect::<HashMap<_, _>>();
                (account_ids, overrides_by_account)
            }
            (None, None) => (Vec::new(), HashMap::new()),
        };

        LaunchOrchestrator::validate_account_ids(&account_ids)?;
        Ok(Self {
            account_ids,
            overrides_by_account,
        })
    }

    pub fn account_ids(&self) -> &[String] {
        &self.account_ids
    }

    pub fn is_empty(&self) -> bool {
        self.account_ids.is_empty()
    }

    pub fn has_overrides(&self) -> bool {
        !self.overrides_by_account.is_empty()
    }

    pub fn override_for(&self, account_id: &str) -> Option<&LaunchAccountOverrides> {
        self.overrides_by_account.get(account_id)
    }

    pub fn apply_for(
        &self,
        requested_account_id: &str,
        account: AccountMeta,
        layout_active: bool,
    ) -> Result<(AccountMeta, Option<LaunchGraphicsOverride>), AppError> {
        let Some(overrides) = self.override_for(requested_account_id) else {
            return Ok((account, None));
        };
        let graphics = validate_graphics_override(overrides)?;
        Ok((
            apply_account_overrides(account, overrides, layout_active)?,
            graphics,
        ))
    }

    pub fn should_persist_position_changes_for(
        &self,
        requested_account_id: &str,
        account: &AccountMeta,
    ) -> bool {
        self.override_for(requested_account_id).is_none() && account.active_position_id.is_some()
    }
}

pub fn validate_graphics_override(
    overrides: &LaunchAccountOverrides,
) -> Result<Option<LaunchGraphicsOverride>, AppError> {
    match (overrides.resolution.as_deref(), overrides.fps) {
        (None, None) => Ok(None),
        (Some(resolution), Some(fps)) => {
            let resolution = resolution.trim();
            let Some((width, height)) = resolution.split_once('x') else {
                return Err(AppError::ConfigReadError(
                    "方案分辨率格式无效，应为 宽x高".to_string(),
                ));
            };
            let width = width.parse::<u32>().map_err(|_| {
                AppError::ConfigReadError("方案分辨率格式无效，应为 宽x高".to_string())
            })?;
            let height = height.parse::<u32>().map_err(|_| {
                AppError::ConfigReadError("方案分辨率格式无效，应为 宽x高".to_string())
            })?;
            if !(800..=7680).contains(&width) || !(600..=4320).contains(&height) {
                return Err(AppError::ConfigReadError(
                    "方案分辨率超出支持范围".to_string(),
                ));
            }
            if fps > 500 {
                return Err(AppError::ConfigReadError(
                    "方案 FPS 必须在 0 到 500 之间".to_string(),
                ));
            }
            Ok(Some(LaunchGraphicsOverride {
                game_settings: overrides
                    .snapshot
                    .as_ref()
                    .map(|snapshot| snapshot.game_settings.clone()),
                resolution: resolution.to_string(),
                fps,
            }))
        }
        _ => Err(AppError::ConfigReadError(
            "方案分辨率与 FPS 必须同时配置".to_string(),
        )),
    }
}

fn apply_account_overrides(
    mut account: AccountMeta,
    overrides: &LaunchAccountOverrides,
    layout_active: bool,
) -> Result<AccountMeta, AppError> {
    let mod_args = overrides.mod_args.trim().to_string();
    // Schemes select from the shared Mod catalog, not the account's historical
    // mod_list. The launch adapter checks the effective installation on disk.
    account.mod_args = mod_args;

    if let Some(snapshot) = &overrides.snapshot {
        let saved =
            crate::domain::account::GameRegion::parse(snapshot.region.as_deref().unwrap_or(""))?;
        let current = account
            .region
            .as_deref()
            .map(crate::domain::account::GameRegion::parse)
            .transpose()?
            .unwrap_or(saved);
        if saved.edition() != current.edition() || snapshot.game_settings.is_empty() {
            return Err(AppError::ConfigReadError(
                "账号身份已变化或方案快照无效，请编辑并保存方案".into(),
            ));
        }
        if saved != current
            && crate::domain::account::AuthMode::parse(account.auth_mode.as_deref())?
                != crate::domain::account::AuthMode::Token
        {
            return Err(AppError::ConfigReadError(
                "战网账号区服已变化，请重新编辑方案；仅 Token 账号支持独立区服".into(),
            ));
        }
        // The scheme supplies the complete game settings. Never read/copy the
        // account's mutable Settings.json before applying this snapshot.
        account.has_customized_settings = false;
        account.region = snapshot.region.clone();
        account.language = snapshot.language.clone();
        account.voicelanguage = snapshot.voicelanguage.clone();
        account.window_x = snapshot.position.map(|position| position.0);
        account.window_y = snapshot.position.map(|position| position.1);
        account.active_position_id = None;
        return Ok(account);
    }

    // Layout geometry supersedes a scheme's position capsule, including a
    // stale/deleted reference. Keep the account default intact for later use.
    if layout_active {
        return Ok(account);
    }

    let position_id = overrides
        .position_preset_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty());
    if let Some(position_id) = position_id {
        let preset = account
            .position_presets
            .iter()
            .find(|preset| preset.id == position_id)
            .ok_or_else(|| {
                AppError::ConfigReadError(format!(
                    "账号 {} 的方案位置已从胶囊库删除，请先修复启动方案",
                    account.id
                ))
            })?;
        account.window_x = Some(preset.x);
        account.window_y = Some(preset.y);
        account.active_position_id = Some(preset.id.clone());
    } else {
        account.window_x = None;
        account.window_y = None;
        account.active_position_id = None;
    }

    Ok(account)
}

pub fn launch_queue_can_continue(success: bool, mutex_closed: bool) -> bool {
    success && mutex_closed
}

#[cfg(test)]
mod tests {
    use super::{
        apply_account_overrides, launch_queue_can_continue, validate_graphics_override,
        LaunchAccountEntry, LaunchAccountOverrides, LaunchBatchPlan,
    };
    use crate::domain::account::{AccountMeta, WindowPositionPreset};

    fn account() -> AccountMeta {
        let mut account = AccountMeta::new("acount1");
        account.mod_args = "-mod default".to_string();
        account.mod_list = vec!["-mod default".to_string(), "-mod plan".to_string()];
        account.active_position_id = Some("desk".to_string());
        account.position_presets = vec![WindowPositionPreset {
            id: "desk".to_string(),
            name: "Desk".to_string(),
            x: 120,
            y: 240,
        }];
        account
    }

    #[test]
    fn snapshot_survives_deleted_position_and_changed_account_defaults() {
        let mut account = account();
        account.region = Some("KR".into());
        account.auth_mode = Some("token".into());
        account.position_presets.clear();
        let original = account.clone();
        let mut settings = serde_json::Map::new();
        settings.insert("Texture Quality".into(), serde_json::json!(3));
        let mut requested = overrides();
        requested.snapshot = Some(crate::domain::launch_scheme::AccountLaunchSnapshot {
            game_settings: settings.clone(),
            position: Some((-100, 50)),
            region: Some("KR".into()),
            language: Some("zhTW".into()),
            voicelanguage: Some("enUS".into()),
        });
        let effective = apply_account_overrides(account, &requested, false).unwrap();
        assert_eq!(effective.window_x, Some(-100));
        assert_eq!(effective.window_y, Some(50));
        assert_ne!(effective.mod_args, original.mod_args);
        assert!(!effective.has_customized_settings);
        assert_eq!(
            validate_graphics_override(&requested)
                .unwrap()
                .unwrap()
                .game_settings,
            Some(settings)
        );
    }

    fn overrides() -> LaunchAccountOverrides {
        LaunchAccountOverrides {
            snapshot: None,
            mod_args: " -mod plan ".to_string(),
            position_preset_id: Some("desk".to_string()),
            resolution: Some("1920x1080".to_string()),
            fps: Some(144),
        }
    }

    #[test]
    fn request_shape_and_case_aliases_are_rejected_before_mapping() {
        assert!(LaunchBatchPlan::from_request(Some(vec!["acount1".into()]), Some(vec![])).is_err());

        let id = "550e8400-e29b-41d4-a716-446655440000";
        assert!(LaunchBatchPlan::from_request(
            None,
            Some(vec![
                LaunchAccountEntry {
                    account_id: id.to_string(),
                    overrides: overrides(),
                },
                LaunchAccountEntry {
                    account_id: id.to_ascii_uppercase(),
                    overrides: overrides(),
                },
            ]),
        )
        .is_err());
    }

    #[test]
    fn launch_plan_applies_ephemeral_values_without_mutating_defaults() {
        let plan = LaunchBatchPlan::from_request(
            None,
            Some(vec![LaunchAccountEntry {
                account_id: "acount1".to_string(),
                overrides: overrides(),
            }]),
        )
        .unwrap();
        let original = account();
        let (effective, graphics) = plan.apply_for("acount1", original.clone(), false).unwrap();

        assert_eq!(original.mod_args, "-mod default");
        assert_eq!(effective.mod_args, "-mod plan");
        assert_eq!(
            (effective.window_x, effective.window_y),
            (Some(120), Some(240))
        );
        assert_eq!(graphics.unwrap().resolution, "1920x1080");
        assert!(!plan.should_persist_position_changes_for("acount1", &effective));
    }

    #[test]
    fn missing_capsules_and_partial_or_out_of_range_graphics_are_rejected() {
        let mut removed_mod = overrides();
        removed_mod.mod_args = "-mod missing".to_string();
        let plan = LaunchBatchPlan::from_request(
            None,
            Some(vec![LaunchAccountEntry {
                account_id: "acount1".to_string(),
                overrides: removed_mod,
            }]),
        )
        .unwrap();
        // Shared Mod catalog availability is checked by the launch adapter
        // against the effective installation; applying a plan only normalizes
        // the transient arguments.
        assert!(plan.apply_for("acount1", account(), false).is_ok());

        let mut partial = overrides();
        partial.fps = None;
        assert!(validate_graphics_override(&partial).is_err());
        let mut oversized = overrides();
        oversized.resolution = Some("9000x1080".to_string());
        assert!(validate_graphics_override(&oversized).is_err());
        let mut fast = overrides();
        fast.fps = Some(501);
        assert!(validate_graphics_override(&fast).is_err());
    }

    #[test]
    fn default_launch_preserves_position_tracking_and_queue_requires_both_signals() {
        let plan = LaunchBatchPlan::from_request(Some(vec!["acount1".into()]), None).unwrap();
        assert!(plan.should_persist_position_changes_for("acount1", &account()));
        assert!(launch_queue_can_continue(true, true));
        assert!(!launch_queue_can_continue(true, false));
        assert!(!launch_queue_can_continue(false, true));
        assert!(!launch_queue_can_continue(false, false));
    }

    #[test]
    fn layout_priority_bypasses_deleted_position_capsules_without_changing_defaults() {
        let mut transient = overrides();
        transient.position_preset_id = Some("deleted".into());
        let plan = LaunchBatchPlan::from_request(
            None,
            Some(vec![LaunchAccountEntry {
                account_id: "acount1".into(),
                overrides: transient,
            }]),
        )
        .unwrap();
        assert!(plan.apply_for("acount1", account(), false).is_err());
        let original = account();
        let (effective, _) = plan.apply_for("acount1", original.clone(), true).unwrap();
        assert_eq!(effective.active_position_id, original.active_position_id);
        assert_eq!(effective.position_presets, original.position_presets);
    }
}
