//! Scheme writes are explicit. Launch projects a copy into the working module
//! settings; those modules have no path back to the saved recipe.
use crate::{
    commands::{account::AccountManager, global_config::mutate_loaded_global_config},
    domain::{
        config::LaunchGroup,
        launch_scheme::{LaunchSession, SchemeEnvironment},
    },
    error::AppError,
    state::SharedState,
};
use tauri::Manager;

pub(crate) fn current_member(
    state: &SharedState,
    id: &str,
) -> Option<crate::domain::config::LaunchGroupMember> {
    let config = state.configuration().snapshot()?;
    let session = config.launch_session?;
    if !state
        .multi_instance()
        .instances()
        .list()
        .iter()
        .any(|instance| session.scheme.account_ids.contains(&instance.account_id))
    {
        return None;
    }
    session
        .members
        .into_iter()
        .find(|member| member.account_id == id)
}

pub(crate) fn update_current_member(
    state: &SharedState,
    app: &tauri::AppHandle,
    id: &str,
    update: impl FnOnce(&mut crate::domain::config::LaunchGroupMember) -> Result<(), AppError>,
) -> Result<bool, AppError> {
    if current_member(state, id).is_none() {
        return Ok(false);
    }
    let _admission = state
        .launch_admission
        .try_lock()
        .map_err(|_| AppError::ConfigWriteError("启动过程中不能修改账号启动设置".into()))?;
    let session_id = state
        .configuration()
        .snapshot()
        .and_then(|config| config.launch_session)
        .map(|session| session.id);
    let mut applied = false;
    mutate_loaded_global_config(state, app, |config| {
        let session = config
            .launch_session
            .as_mut()
            .filter(|session| Some(&session.id) == session_id.as_ref())
            .ok_or_else(|| AppError::ConfigWriteError("本次运行已变化，请重新打开设置".into()))?;
        let member = session
            .members
            .iter_mut()
            .find(|member| member.account_id == id)
            .ok_or_else(|| AppError::ConfigWriteError("账号不属于本次运行".into()))?;
        update(member)?;
        applied = true;
        Ok(true)
    })?;
    Ok(applied)
}

#[tauri::command]
pub(crate) fn save_launch_scheme(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
    mut scheme: LaunchGroup,
    expected_revision: Option<u64>,
) -> Result<LaunchGroup, AppError> {
    if scheme.id.trim().is_empty() || scheme.name.trim().is_empty() || scheme.members.is_empty() {
        return Err(AppError::ConfigWriteError(
            "请输入方案名称并选择账号".into(),
        ));
    }
    scheme.name = scheme.name.trim().into();
    scheme.account_ids = scheme
        .members
        .iter()
        .map(|member| member.account_id.clone())
        .collect();
    let config = state
        .configuration()
        .snapshot()
        .ok_or_else(|| AppError::ConfigReadError("配置尚未加载".into()))?;
    let environment = scheme
        .environment
        .as_ref()
        .ok_or_else(|| AppError::ConfigWriteError("请补全方案的跟房、识别和布局设置".into()))?;
    environment.validate(&scheme, &config)?;
    let mut ids = std::collections::HashSet::new();
    for member in &mut scheme.members {
        AccountManager::validate_account_id(&member.account_id)?;
        if !ids.insert(&member.account_id) {
            return Err(AppError::ConfigWriteError("方案账号重复".into()));
        }
        let account = AccountManager::load_meta(&config.accounts_dir, &member.account_id)?;
        let snapshot = member
            .snapshot
            .as_mut()
            .ok_or_else(|| AppError::ConfigWriteError("请等待账号设置加载完成".into()))?;
        if snapshot.game_settings.is_empty() || !member.graphics_configured {
            return Err(AppError::ConfigWriteError("账号游戏设置不完整".into()));
        }
        // Installation/identity migration belongs to account management.
        let current_region = crate::launch_context::LaunchContext::for_account(
            &config,
            &account,
            crate::launch_context::ContextPurpose::Settings,
        )?
        .game_region;
        let saved_region = snapshot
            .region
            .as_deref()
            .map(crate::domain::account::GameRegion::parse)
            .transpose()?
            .unwrap_or(current_region);
        snapshot.region = Some(saved_region.canonical().into());
        if saved_region != current_region
            && crate::domain::account::AuthMode::parse(account.auth_mode.as_deref())?
                != crate::domain::account::AuthMode::Token
        {
            return Err(AppError::ConfigWriteError(
                "只有 Token 账号支持在方案中独立选择国际服区服".into(),
            ));
        }
        if saved_region.edition() != current_region.edition() {
            return Err(AppError::ConfigWriteError(
                "方案不能改变账号的国服/国际服身份".into(),
            ));
        }
    }
    scheme.revision = expected_revision
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| AppError::ConfigWriteError("方案版本溢出".into()))?;
    mutate_loaded_global_config(state.inner(), &app, |config| {
        let existing = config
            .launch_groups
            .iter()
            .position(|group| group.id == scheme.id);
        if existing.map(|index| config.launch_groups[index].revision) != expected_revision {
            return Err(AppError::ConfigWriteError(
                "方案已被修改或删除，请重新打开编辑".into(),
            ));
        }
        if let Some(index) = existing {
            config.launch_groups[index] = scheme.clone();
        } else {
            config.launch_groups.push(scheme.clone());
        }
        Ok(true)
    })?;
    Ok(scheme)
}

#[tauri::command]
pub(crate) fn delete_launch_scheme(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
    id: String,
    expected_revision: u64,
) -> Result<(), AppError> {
    mutate_loaded_global_config(state.inner(), &app, |config| {
        let index = config
            .launch_groups
            .iter()
            .position(|group| group.id == id)
            .ok_or_else(|| AppError::ConfigWriteError("方案已删除".into()))?;
        if config.launch_groups[index].revision != expected_revision {
            return Err(AppError::ConfigWriteError("方案已修改，请重新打开".into()));
        }
        config.launch_groups.remove(index);
        config
            .favorite_launch_group_ids
            .retain(|candidate| candidate != &id);
        Ok(true)
    })?;
    Ok(())
}

pub(crate) fn room_manager(
    app: &tauri::AppHandle,
) -> Option<std::sync::Arc<crate::capabilities::room_automation_runtime::RoomAutomationManager>> {
    crate::capabilities::initialize_room_automation_if_installed(app);
    app.try_state::<crate::capabilities::room_automation_runtime::RoomAutomationCommandState>()
        .and_then(|slot| slot.manager().ok().cloned())
}

#[tauri::command]
pub(crate) fn get_launch_scheme_defaults(
    state: tauri::State<'_, SharedState>,
) -> Result<SchemeEnvironment, AppError> {
    let config = state
        .configuration()
        .snapshot()
        .ok_or_else(|| AppError::ConfigReadError("配置尚未加载".into()))?;
    let mut settings = crate::domain::launch_scheme::settings_snapshot(&config);
    settings.insert("rune_audio_enabled".into(), false.into());
    settings.insert("rune_audio_target_account".into(), "".into());
    settings.insert("rune_audio_external_target".into(), serde_json::Value::Null);
    settings.insert("keep_battle_net_account_id".into(), serde_json::Value::Null);
    Ok(SchemeEnvironment {
        settings,
        room: Default::default(),
        layout: None,
    })
}

/// Read-only preflight and the durable working-copy commit happen under one
/// launch admission lease. No frontend supplied entries are trusted here.
pub(crate) fn prepare_scheme_launch(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, SharedState>,
    id: &str,
    revision: u64,
) -> Result<Vec<crate::application::multi_instance::LaunchAccountEntry>, AppError> {
    let _activation = state
        .runtime_activation_lock
        .try_lock()
        .ok_or_else(|| AppError::ConfigReadError("模块配置操作进行中，请稍后重试".into()))?;
    let config = state
        .configuration()
        .snapshot()
        .ok_or_else(|| AppError::ConfigReadError("配置尚未加载".into()))?;
    let scheme = config
        .launch_groups
        .iter()
        .find(|scheme| scheme.id == id && scheme.revision == revision)
        .cloned()
        .ok_or_else(|| AppError::ConfigReadError("方案已修改，请刷新后再启动".into()))?;
    let environment = scheme.environment.as_ref().ok_or_else(|| {
        AppError::ConfigReadError("旧方案需要先编辑并保存，以明确跟房、识别和布局配置".into())
    })?;
    environment.validate(&scheme, &config)?;
    let accounts = crate::commands::account::list_accounts(state.clone())?;
    if accounts.iter().any(|account| account.is_running) {
        return Err(AppError::ConfigReadError(
            "请先关闭当前运行的游戏，再按方案启动；运行期间可以单独调整布局、跟房和识别".into(),
        ));
    }
    let manager = room_manager(app);
    if manager
        .as_ref()
        .is_some_and(|manager| manager.get_status().running)
    {
        return Err(AppError::ConfigReadError("请先结束当前跟房任务".into()));
    }
    if environment.room.enabled
        && (manager.is_none()
            || !config.optional_module_runtime_allowed(
                crate::domain::config::OPTIONAL_MODULE_ROOM_AUTOMATION,
            ))
    {
        return Err(AppError::ConfigReadError(
            "方案需要自动跟房扩展，请先添加该扩展".into(),
        ));
    }
    let mut projected = environment.project(&config)?;
    let session = LaunchSession {
        id: uuid::Uuid::new_v4().to_string(),
        scheme: scheme.clone(),
        started_at: chrono::Utc::now().to_rfc3339(),
        processes: Vec::new(),
        members: scheme.members.clone(),
    };
    projected.launch_session = Some(session.clone());
    if projected.rune_audio_enabled
        && !projected
            .optional_module_runtime_allowed(crate::domain::config::OPTIONAL_MODULE_AUTOMATION)
    {
        return Err(AppError::ConfigReadError(
            "方案需要识别扩展，请先添加该扩展".into(),
        ));
    }
    if (projected.enable_stats_overlay || projected.enable_tz_overlay)
        && !projected
            .optional_module_runtime_allowed(crate::domain::config::OPTIONAL_MODULE_OVERLAYS)
    {
        return Err(AppError::ConfigReadError(
            "方案需要悬浮窗扩展，请先添加扩展或在方案中关闭悬浮窗".into(),
        ));
    }
    let entries = scheme
        .members
        .iter()
        .map(|member| {
            if member.snapshot.is_none() {
                return Err(AppError::ConfigReadError(
                    "方案缺少账号快照，请重新编辑保存".into(),
                ));
            }
            Ok(crate::application::multi_instance::LaunchAccountEntry {
                account_id: member.account_id.clone(),
                overrides: crate::application::multi_instance::LaunchAccountOverrides {
                    snapshot: member.snapshot.clone(),
                    mod_args: member.mod_args.clone().unwrap_or_default(),
                    position_preset_id: None,
                    resolution: member.resolution.clone(),
                    fps: member.fps,
                },
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    crate::commands::launch::preflight_scheme_entries(&projected, state.inner(), &entries)?;
    let previous_room = manager.as_ref().map(|manager| manager.get_config());
    crate::commands::launch_scheme_recovery::write(
        state.inner(),
        Some(crate::commands::launch_scheme_recovery::PendingApply {
            session_id: session.id.clone(),
            previous_room: previous_room
                .as_ref()
                .map(|snapshot| snapshot.config.clone()),
        }),
    )?;
    // Commit room first; on any later failure restore only the fields this
    // operation owns. Configuration mutations merge with the latest snapshot.
    if let Some(manager) = &manager {
        let outcome = manager
            .save_config(manager.get_config().generation, environment.room.clone())
            .map_err(AppError::ConfigWriteError)?;
        if let Some(warning) = outcome.apply_warning {
            if let Some(previous) = previous_room {
                manager
                    .save_config(outcome.snapshot.generation, previous.config)
                    .map_err(AppError::ConfigWriteError)?;
            }
            crate::commands::launch_scheme_recovery::write(state.inner(), None)?;
            return Err(AppError::ConfigWriteError(warning));
        }
    }
    let result = mutate_loaded_global_config(state.inner(), app, |current| {
        let mut next = environment.project(current)?;
        next.launch_session = Some(session);
        *current = next;
        Ok(true)
    });
    if let Err(error) = result {
        if let (Some(manager), Some(previous)) = (manager, previous_room) {
            manager
                .save_config(manager.get_config().generation, previous.config)
                .map_err(|rollback| {
                    AppError::ConfigWriteError(format!("{error}；恢复跟房设置失败：{rollback}"))
                })?;
        }
        crate::commands::launch_scheme_recovery::write(state.inner(), None)?;
        return Err(error);
    }
    crate::commands::launch_scheme_recovery::write(state.inner(), None)?;
    Ok(entries)
}
