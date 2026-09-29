//! Fail-closed handshake for every Hub-managed processor invocation.
use crate::infrastructure::managed_process;
use serde::Deserialize;
use std::{path::Path, time::Duration};

pub(crate) const CONTRACT: &str = "d2rhub-processing-v1-r32";

pub(crate) fn command(path: &Path) -> tokio::process::Command {
    let mut command = managed_process::command(path);
    command.env("D2RHUB_VERSION", env!("CARGO_PKG_VERSION"));
    command.env("D2RHUB_PROCESSING_CONTRACT", CONTRACT);
    command
}

#[derive(Deserialize)]
struct Identity {
    product: String,
    version: String,
    contract: String,
    required_hub: String,
}

fn validate(identity: &Identity, expected_version: &str) -> Result<(), String> {
    if identity.product != "d2r-audio-mod"
        || identity.version != expected_version
        || identity.contract != CONTRACT
        || identity.required_hub != env!("CARGO_PKG_VERSION")
    {
        return Err(format!(
            "配套校验失败：当前 D2RHub {}，加工器 {}；该加工器要求 D2RHub {}。请在“下载与更新”安装配套加工器 {}；若加工器要求不同 Hub 版本，请更新 D2RHub。互认成功前禁止加工。",
            env!("CARGO_PKG_VERSION"), identity.version, identity.required_hub, expected_version
        ));
    }
    Ok(())
}

pub(crate) async fn verify(path: &Path, expected_version: &str) -> Result<(), String> {
    let mut command = command(path);
    command.arg("hub-compatibility");
    let mut identity = None;
    let output = managed_process::run(
        &mut command,
        Some(Duration::from_secs(5)),
        || false,
        |line| {
            if identity.is_some() {
                return Err("加工器返回了重复的配套身份信息".into());
            }
            identity = Some(serde_json::from_slice::<Identity>(line)
                .map_err(|_| "加工器配套身份信息无效".to_string())?);
            Ok(())
        },
    ).await.map_err(|error| format!("加工器互认失败，已禁止加工：{error}。请在“下载与更新”更新加工器；若仍失败，请同时更新 D2RHub。"))?;
    if output.exit_code != Some(0) {
        return Err(format!("D2RHub {} 与加工器 {expected_version} 未能互认，已禁止加工。请在“下载与更新”安装配套加工器，或按要求更新 D2RHub。{}", env!("CARGO_PKG_VERSION"), output.stderr.trim()));
    }
    validate(
        &identity.ok_or("加工器未返回配套身份，已禁止加工。请更新加工器和 D2RHub。")?,
        expected_version,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairing_requires_every_identity_field() {
        let identity = || Identity {
            product: "d2r-audio-mod".into(),
            version: "1.4.0-beta.19".into(),
            contract: CONTRACT.into(),
            required_hub: env!("CARGO_PKG_VERSION").into(),
        };
        assert!(validate(&identity(), "1.4.0-beta.19").is_ok());
        assert!(validate(&identity(), "1.4.0-beta.20").is_err());
        let mut bad = identity();
        bad.product = "other".into();
        assert!(validate(&bad, "1.4.0-beta.19").is_err());
        let mut bad = identity();
        bad.contract = "old".into();
        assert!(validate(&bad, "1.4.0-beta.19").is_err());
        let mut bad = identity();
        bad.required_hub = "0.9.109".into();
        assert!(validate(&bad, "1.4.0-beta.19").is_err());
    }
}
