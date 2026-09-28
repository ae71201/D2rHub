//! Mod catalog identities, reference planning and persisted data contracts.
//! Receives account snapshots and canonical installation identities; performs
//! no I/O and has no dependency on Tauri commands or processor runtime code.
use crate::domain::account::{AccountMeta, GameRegion};
use crate::domain::config::GlobalConfig;
use crate::domain::mod_arguments::{active_mod_name, parse_windows_command_line};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

const MAX_ARGUMENT_LENGTH: usize = 2_048;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct ModCatalogPayload {
    pub(crate) argument_overrides: BTreeMap<String, String>,
    pub(crate) custom_entries: Vec<CustomModEntry>,
    pub(crate) legacy_import_completed: bool,
    pub(crate) pending_argument_update: Option<PendingCatalogArgumentUpdate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PendingCatalogArgumentUpdate {
    pub(crate) capsule_id: String,
    pub(crate) accounts: Vec<AccountModJournalEntry>,
    pub(crate) scheme_members: Vec<SchemeModJournalEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AccountModJournalEntry {
    pub(crate) account_id: String,
    pub(crate) old_active: String,
    pub(crate) old_list: Vec<String>,
    pub(crate) new_active: String,
    pub(crate) new_list: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SchemeModJournalEntry {
    pub(crate) group_id: String,
    pub(crate) account_id: String,
    pub(crate) old_arguments: Option<String>,
    pub(crate) new_arguments: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CustomModEntry {
    pub(crate) id: String,
    pub(crate) edition: String,
    pub(crate) launch_arguments: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModCapsule {
    pub id: String,
    pub edition: String,
    pub name: String,
    pub origin: String,
    pub launch_arguments: String,
    pub default_launch_arguments: Option<String>,
    pub source_mod_name: Option<String>,
    pub lightweight_profile: Option<String>,
    pub issue: Option<String>,
    pub feature_groups: Vec<String>,
    pub auto_exit_on_death_enabled: bool,
    pub processed: bool,
    pub source_eligible: bool,
    pub update_required: bool,
    pub ready: bool,
    pub deletable: bool,
    pub assigned_account_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModCapsuleAccountSelection {
    pub account_id: String,
    pub account_name: String,
    pub edition: Option<String>,
    pub selected_capsule_id: Option<String>,
    pub legacy_mod_arguments: String,
    pub issue: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModCapsulePool {
    pub generation: u64,
    pub scanned_at: String,
    pub capsules: Vec<ModCapsule>,
    pub accounts: Vec<ModCapsuleAccountSelection>,
}

#[derive(Clone)]
pub(crate) struct AccountModReplacement {
    pub(crate) original: AccountMeta,
    pub(crate) active: String,
    pub(crate) list: Vec<String>,
}

impl AccountModReplacement {
    pub(crate) fn journal_entry(&self) -> AccountModJournalEntry {
        AccountModJournalEntry {
            account_id: self.original.id.clone(),
            old_active: self.original.mod_args.clone(),
            old_list: self.original.mod_list.clone(),
            new_active: self.active.clone(),
            new_list: self.list.clone(),
        }
    }
}

pub(crate) fn normalize_edition(value: &str) -> Result<String, String> {
    match value.trim().to_ascii_uppercase().as_str() {
        "CN" => Ok("CN".to_string()),
        "GLOBAL" => Ok("Global".to_string()),
        _ => Err("Mod 胶囊版本只能是 CN 或 Global".to_string()),
    }
}

pub(crate) fn account_edition(config: &GlobalConfig, account: &AccountMeta) -> Option<String> {
    if let Some(region) = account.region.as_deref() {
        return GameRegion::parse(region)
            .ok()
            .map(|region| region.edition().canonical().to_string());
    }
    match (
        !config.cn_game_path.trim().is_empty(),
        !config.global_game_path.trim().is_empty(),
    ) {
        (true, false) => Some("CN".to_string()),
        (false, true) => Some("Global".to_string()),
        _ => None,
    }
}

pub(crate) fn scanned_capsule_id(edition: &str, name: &str) -> String {
    format!(
        "scan:{}:{}",
        edition.to_ascii_lowercase(),
        name.trim().to_ascii_lowercase()
    )
}

pub(crate) fn validate_arguments(arguments: &str) -> Result<String, String> {
    let arguments = arguments.trim();
    if arguments.is_empty() {
        return Err("自定义 Mod 参数不能为空；原版游戏请直接选择“不使用 Mod”".to_string());
    }
    if arguments.len() > MAX_ARGUMENT_LENGTH {
        return Err(format!("Mod 参数不能超过 {MAX_ARGUMENT_LENGTH} 个字符"));
    }
    parse_windows_command_line(arguments).map_err(|error| format!("Mod 参数无法解析：{error}"))?;
    Ok(arguments.to_string())
}

pub(crate) fn custom_display_name(arguments: &str) -> String {
    active_mod_name(arguments)
        .ok()
        .flatten()
        .unwrap_or_else(|| "自定义参数".to_string())
}

pub(crate) fn arguments_reference_capsule(capsule: &ModCapsule, arguments: &str) -> bool {
    if capsule.origin == "scanned" {
        // A scanned entry owns a physical folder. Different flags or casing do
        // not make a second folder safe to delete.
        active_mod_name(arguments)
            .ok()
            .flatten()
            .is_some_and(|name| name.eq_ignore_ascii_case(&capsule.name))
    } else {
        arguments.trim() == capsule.launch_arguments.trim()
    }
}

pub(crate) fn plan_catalog_argument_replacements(
    config: &GlobalConfig,
    accounts: &[AccountMeta],
    edition: &str,
    old_arguments: &str,
    new_arguments: &str,
) -> Vec<AccountModReplacement> {
    let mut changes = Vec::new();
    for account in accounts {
        if account_edition(config, account).as_deref() != Some(edition) {
            continue;
        }
        let active = if account.mod_args.trim() == old_arguments {
            new_arguments.to_string()
        } else {
            account.mod_args.clone()
        };
        let mut changed = active != account.mod_args;
        let list = account
            .mod_list
            .iter()
            .map(|arguments| {
                if arguments.trim() == old_arguments {
                    changed = true;
                    new_arguments.to_string()
                } else {
                    arguments.clone()
                }
            })
            .collect::<Vec<_>>();
        if changed {
            let mut normalized = account.clone();
            normalized.replace_mod_configurations(active, list);
            changes.push(AccountModReplacement {
                original: account.clone(),
                active: normalized.mod_args,
                list: normalized.mod_list,
            });
        }
    }
    changes
}

pub(crate) fn plan_catalog_argument_replacements_in_schemes(
    config: &GlobalConfig,
    accounts: &[AccountMeta],
    edition: &str,
    old_arguments: &str,
    new_arguments: &str,
) -> Vec<SchemeModJournalEntry> {
    let mut account_editions = HashMap::new();
    for account in accounts {
        let account_edition = account_edition(config, account);
        account_editions.insert(account.id.to_ascii_lowercase(), account_edition);
    }
    config
        .launch_groups
        .iter()
        .flat_map(|group| {
            group
                .members
                .iter()
                .filter(|member| {
                    account_editions
                        .get(&member.account_id.to_ascii_lowercase())
                        .and_then(Option::as_deref)
                        == Some(edition)
                        && member
                            .mod_args
                            .as_deref()
                            .is_some_and(|arguments| arguments.trim() == old_arguments)
                })
                .map(|member| SchemeModJournalEntry {
                    group_id: group.id.clone(),
                    account_id: member.account_id.clone(),
                    old_arguments: member.mod_args.clone(),
                    new_arguments: Some(new_arguments.to_string()),
                })
        })
        .collect()
}

/// Canonical physical identities supplied by the filesystem adapter. Missing
/// identities stay unknown and therefore cannot authorize destructive edits.
#[derive(Default)]
pub(crate) struct InstallationIdentities {
    pub(crate) cn: Option<String>,
    pub(crate) global: Option<String>,
}

impl InstallationIdentities {
    fn get(&self, edition: &str) -> Option<&str> {
        match edition {
            "CN" => self.cn.as_deref(),
            "Global" => self.global.as_deref(),
            _ => None,
        }
    }
}

fn capsule_can_be_used_by_edition(
    identities: &InstallationIdentities,
    capsule: &ModCapsule,
    edition: Option<&str>,
) -> bool {
    let Some(edition) = edition else {
        return true;
    };
    if edition == capsule.edition {
        return true;
    }
    if capsule.origin != "scanned" {
        return false;
    }
    match (identities.get(edition), identities.get(&capsule.edition)) {
        (Some(account_root), Some(capsule_root)) => account_root == capsule_root,
        _ => true,
    }
}

pub(crate) fn capsule_usage(
    config: &GlobalConfig,
    capsule: &ModCapsule,
    accounts: &[AccountMeta],
    identities: &InstallationIdentities,
) -> Vec<String> {
    let mut usage = Vec::new();
    let mut account_editions = HashMap::new();
    for account in accounts {
        let edition = account_edition(config, account);
        // An unresolved legacy account can reference either installation;
        // retain its Mod until the account edition is explicitly configured.
        if capsule_can_be_used_by_edition(identities, capsule, edition.as_deref())
            && arguments_reference_capsule(capsule, &account.mod_args)
        {
            usage.push(if account.display_name.trim().is_empty() {
                account.id.clone()
            } else {
                account.display_name.clone()
            });
        }
        account_editions.insert(account.id.to_ascii_lowercase(), edition);
    }
    for group in &config.launch_groups {
        if group.members.iter().any(|member| {
            capsule_can_be_used_by_edition(
                identities,
                capsule,
                account_editions
                    .get(&member.account_id.to_ascii_lowercase())
                    .and_then(Option::as_deref),
            ) && member
                .mod_args
                .as_deref()
                .is_some_and(|value| arguments_reference_capsule(capsule, value))
        }) {
            usage.push(format!("启动方案：{}", group.name));
        }
    }
    usage
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanned_ids_are_case_insensitive_but_edition_scoped() {
        assert_eq!(
            scanned_capsule_id("Global", "MyMod"),
            scanned_capsule_id("Global", "mymod")
        );
        assert_ne!(
            scanned_capsule_id("CN", "MyMod"),
            scanned_capsule_id("Global", "MyMod")
        );
    }

    #[test]
    fn scanned_argument_edits_cannot_change_the_folder_identity() {
        let accepted = validate_arguments("-mod Sample -txt -assettestmode 1 -foo").unwrap();
        assert_eq!(
            active_mod_name(&accepted).unwrap().as_deref(),
            Some("Sample")
        );
        assert_eq!(
            active_mod_name("-mod Other -txt").unwrap().as_deref(),
            Some("Other")
        );
    }

    #[test]
    fn legacy_sidecar_fields_and_default_migration_state_remain_compatible() {
        let payload: ModCatalogPayload = serde_json::from_str(
            r#"{
            "argument_overrides": {"scan:cn:sample":"-mod Sample -txt"},
            "custom_entries": [{"id":"custom:one","edition":"CN","launch_arguments":"-mod Other"}]
        }"#,
        )
        .unwrap();
        assert!(!payload.legacy_import_completed);
        assert!(payload.pending_argument_update.is_none());
        let value = serde_json::to_value(payload).unwrap();
        assert_eq!(
            value["argument_overrides"]["scan:cn:sample"],
            "-mod Sample -txt"
        );
        assert_eq!(value["custom_entries"][0]["id"], "custom:one");
    }

    #[test]
    fn replacement_plans_preserve_inputs_and_unrelated_account_fields() {
        let config = GlobalConfig::default();
        let mut account = AccountMeta::new("acount1");
        account.region = Some("CN".into());
        account.display_name = "Keep my name".into();
        account.mod_args = "-mod old".into();
        account.mod_list = vec!["-mod old".into(), "-mod unrelated".into()];
        let planned = plan_catalog_argument_replacements(
            &config,
            std::slice::from_ref(&account),
            "CN",
            "-mod old",
            "-mod new",
        );
        assert_eq!(planned.len(), 1);
        assert_eq!(account.mod_args, "-mod old");
        assert_eq!(planned[0].original.display_name, "Keep my name");
        assert_eq!(planned[0].active, "-mod new");
        assert_eq!(planned[0].list, vec!["-mod new", "-mod unrelated"]);
        assert!(plan_catalog_argument_replacements(
            &config,
            &[account],
            "Global",
            "-mod old",
            "-mod new"
        )
        .is_empty());
    }
}
