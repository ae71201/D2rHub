//! The global session ID is the commit point of the room/global handoff.
//! A crash before that point restores the previous room settings before any
//! optional service is installed. Recovery never launches games or sends keys.
use crate::{
    domain::room_automation::RoomAutomationConfig, error::AppError,
    infrastructure::module_config::ModuleConfigStore, state::SharedState,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PendingApply {
    pub session_id: String,
    pub previous_room: Option<RoomAutomationConfig>,
}

fn store(state: &SharedState) -> Result<ModuleConfigStore, AppError> {
    ModuleConfigStore::new(&state.app_data_dir, "launch-scheme-transition", 1)
        .map_err(|error| AppError::ConfigWriteError(error.to_string()))
}

pub(crate) fn write(state: &SharedState, pending: Option<PendingApply>) -> Result<(), AppError> {
    let store = store(state)?;
    let current = store
        .load::<Option<PendingApply>>()
        .map_err(|error| AppError::ConfigReadError(error.to_string()))?;
    store
        .save_if_generation(current.map_or(0, |value| value.generation), pending)
        .map_err(|error| AppError::ConfigWriteError(error.to_string()))?;
    Ok(())
}

pub(crate) fn recover(state: &SharedState) -> Result<(), AppError> {
    let Some(pending) = store(state)?
        .load::<Option<PendingApply>>()
        .map_err(|error| AppError::ConfigReadError(error.to_string()))?
        .and_then(|value| value.payload)
    else {
        return Ok(());
    };
    let committed = state
        .configuration()
        .snapshot()
        .and_then(|config| config.launch_session)
        .is_some_and(|session| session.id == pending.session_id);
    if !committed {
        if let Some(room) = pending.previous_room {
            let controller =
                crate::capabilities::room_automation_config::RoomAutomationConfigController::new(
                    &state.app_data_dir,
                )
                .map_err(|error| AppError::ConfigReadError(error.to_string()))?;
            let snapshot = controller
                .load_or_initialize(None, &[])
                .map_err(|error| AppError::ConfigReadError(error.to_string()))?;
            controller
                .save(snapshot.generation, room, &[])
                .map_err(|error| AppError::ConfigWriteError(error.to_string()))?;
        }
    }
    write(state, None)
}

pub(crate) fn capture_process(
    state: &SharedState,
    app: &tauri::AppHandle,
    session_id: &str,
    account_id: &str,
    pid: u32,
    mod_args: &str,
) -> Result<bool, AppError> {
    let Some(window) = crate::infrastructure::game_layout_windows::snapshot()?
        .into_iter()
        .find(|window| window.pid == pid && window.started_at != 0)
    else {
        return Ok(false);
    };
    crate::commands::global_config::mutate_loaded_global_config(state, app, |config| {
        let Some(session) = config
            .launch_session
            .as_mut()
            .filter(|session| session.id == session_id)
        else {
            return Ok(false);
        };
        session
            .processes
            .retain(|process| process.account_id != account_id);
        session
            .processes
            .push(crate::domain::launch_scheme::SessionProcess {
                account_id: account_id.into(),
                pid,
                started_at: window.started_at,
                executable: window.executable,
                mod_args: mod_args.into(),
            });
        Ok(true)
    })?;
    Ok(true)
}

pub(crate) fn recover_processes(
    state: &SharedState,
    app: &tauri::AppHandle,
) -> Result<(), AppError> {
    let Some(session) = state
        .configuration()
        .snapshot()
        .and_then(|config| config.launch_session)
    else {
        return Ok(());
    };
    let observed = crate::infrastructure::game_layout_windows::snapshot()?;
    let mut restored = 0;
    for process in &session.processes {
        if observed
            .iter()
            .any(|window| matches_process(process, window))
        {
            state.multi_instance().instances().record_launched(
                &process.account_id,
                process.pid,
                &process.mod_args,
            );
            restored += 1;
        }
    }
    if restored == 0 {
        crate::commands::global_config::mutate_loaded_global_config(state, app, |config| {
            config.launch_session = None;
            Ok(true)
        })?;
    }
    Ok(())
}

fn matches_process(
    process: &crate::domain::launch_scheme::SessionProcess,
    window: &crate::infrastructure::game_layout_windows::GameLayoutWindow,
) -> bool {
    window.pid == process.pid
        && window.started_at != 0
        && window.started_at == process.started_at
        && crate::infrastructure::system::executable_paths_match(
            &window.executable,
            &process.executable,
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{launch_scheme::SessionProcess, window_layout::LayoutRect};
    use crate::infrastructure::game_layout_windows::GameLayoutWindow;

    #[test]
    fn reused_pid_or_different_installation_cannot_restore_a_session() {
        let saved = SessionProcess {
            account_id: "a".into(),
            pid: 42,
            started_at: 100,
            executable: std::path::PathBuf::from(r"C:\Game\D2R.exe"),
            mod_args: "-mod Saved".into(),
        };
        let mut observed = GameLayoutWindow {
            handle: 1,
            pid: 42,
            title: "a".into(),
            executable: saved.executable.clone(),
            started_at: 100,
            rect: LayoutRect {
                x: 0,
                y: 0,
                width: 1280,
                height: 720,
            },
            minimized: false,
        };
        assert!(matches_process(&saved, &observed));
        observed.started_at = 101;
        assert!(!matches_process(&saved, &observed));
        observed.started_at = 0;
        assert!(!matches_process(&saved, &observed));
        observed.started_at = 100;
        observed.executable = std::path::PathBuf::from(r"C:\OtherGame\D2R.exe");
        assert!(!matches_process(&saved, &observed));
    }

    #[test]
    fn interrupted_room_handoff_rolls_back_before_services_start() {
        let root = std::env::temp_dir().join(format!("scheme-recovery-{}", uuid::Uuid::new_v4()));
        let mut state = crate::state::AppState::new();
        state.app_data_dir = root.to_string_lossy().into_owned();
        let state = std::sync::Arc::new(state);
        let controller =
            crate::capabilities::room_automation_config::RoomAutomationConfigController::new(&root)
                .unwrap();
        let before = controller.load_or_initialize(None, &[]).unwrap();
        let mut after = before.config.clone();
        after.name_prefix = "replacement".into();
        write(
            &state,
            Some(PendingApply {
                session_id: "not-committed".into(),
                previous_room: Some(before.config.clone()),
            }),
        )
        .unwrap();
        controller.save(before.generation, after, &[]).unwrap();
        recover(&state).unwrap();
        assert_eq!(
            controller.load_or_initialize(None, &[]).unwrap().config,
            before.config
        );
        assert!(store(&state)
            .unwrap()
            .load::<Option<PendingApply>>()
            .unwrap()
            .unwrap()
            .payload
            .is_none());
        recover(&state).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
