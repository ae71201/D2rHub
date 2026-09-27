use crate::infrastructure::module_config::ModuleConfigStore;
use crate::state::SharedState;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

// Keep in sync with APPLICATION_DISCLOSURE_REVISION; bump only when the notice changes.
const CURRENT_REVISION: u32 = 1;
static DISCLOSURE_IO: Mutex<()> = Mutex::new(());

#[derive(Deserialize, Serialize)]
struct Acceptance {
    revision: u32,
}

fn transact(directory: &str, accept: bool) -> Result<Option<u32>, String> {
    let _guard = DISCLOSURE_IO.lock();
    let store = ModuleConfigStore::new(directory, "application-disclosure", 1)
        .map_err(|error| error.to_string())?;
    let existing = store
        .load::<Acceptance>()
        .map_err(|error| error.to_string())?;
    if !accept {
        return Ok(existing.map(|value| value.payload.revision));
    }
    if existing
        .as_ref()
        .is_some_and(|value| value.payload.revision == CURRENT_REVISION)
    {
        return Ok(Some(CURRENT_REVISION));
    }
    let generation = existing.map_or(0, |value| value.generation);
    store
        .save_if_generation(
            generation,
            Acceptance {
                revision: CURRENT_REVISION,
            },
        )
        .map_err(|error| error.to_string())?;
    Ok(Some(CURRENT_REVISION))
}

#[tauri::command(async)]
pub fn get_application_disclosure_acceptance(
    state: tauri::State<'_, SharedState>,
) -> Result<Option<u32>, String> {
    transact(&state.app_data_dir, false)
}

#[tauri::command(async)]
pub fn accept_application_disclosure(
    state: tauri::State<'_, SharedState>,
    revision: u32,
) -> Result<(), String> {
    if revision != CURRENT_REVISION {
        return Err("使用须知修订号不匹配，请重新启动 D2RHub".into());
    }
    transact(&state.app_data_dir, true).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acceptance_survives_reopening_and_repeated_acceptance() {
        let root = std::env::temp_dir().join(format!(
            "disclosure-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let directory = root.to_str().unwrap();
        assert_eq!(transact(directory, false).unwrap(), None);
        assert_eq!(transact(directory, true).unwrap(), Some(CURRENT_REVISION));
        assert_eq!(transact(directory, false).unwrap(), Some(CURRENT_REVISION));
        assert_eq!(transact(directory, true).unwrap(), Some(CURRENT_REVISION));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unreadable_storage_is_an_error_not_missing_consent() {
        let root = std::env::temp_dir().join(format!(
            "disclosure-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        std::fs::write(&root, "blocked directory").unwrap();
        assert!(transact(root.to_str().unwrap(), false).is_err());
        assert!(transact(root.to_str().unwrap(), true).is_err());
        std::fs::remove_file(root).unwrap();
    }
}
