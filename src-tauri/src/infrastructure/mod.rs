//! Required platform adapters shared by the core and optional capabilities.

pub(crate) mod account_process;
pub(crate) mod account_rename;
pub(crate) mod diagnostics;
pub(crate) mod durable_fs;
pub(crate) mod game_layout_windows;
pub(crate) mod managed_process;
#[cfg(target_os = "windows")]
pub(crate) mod memory_trim;
pub mod module_config;
#[cfg(target_os = "windows")]
pub(crate) mod physical_input;
pub(crate) mod process;
pub(crate) mod system;
