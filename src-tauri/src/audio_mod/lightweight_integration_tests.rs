//! Opt-in integration matrix: independent processor, original game, Hub validators.
use super::*;
use crate::domain::mod_processing::GeneratorReport;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

#[test]
#[ignore = "requires a BoHub source with empty sound references and original game resources"]
fn silent_sound_references_can_be_processed_without_restoring_original_audio() {
    let game = std::env::var("D2RHUB_LIGHTWEIGHT_GAME_ROOT").unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../artifacts")
        .join(format!("silent-source-{}", uuid::Uuid::new_v4()));
    let source = root.join("Silent");
    let excel = source.join("Silent.mpq/data/global/excel");
    std::fs::create_dir_all(&excel).unwrap();
    let sounds =
        std::fs::read(Path::new(&game).join("mods/BoHub/BoHub.mpq/data/global/excel/sounds.txt"))
            .unwrap();
    std::fs::write(excel.join("sounds.txt"), &sounds).unwrap();
    std::fs::write(
        source.join("Silent.mpq/modinfo.json"),
        r#"{"name":"Silent","savepath":"Silent/"}"#,
    )
    .unwrap();
    let value = invoke(
        &[
            "augment".into(),
            "--game".into(),
            game,
            "--source".into(),
            source.display().to_string(),
            "--output".into(),
            root.display().to_string(),
            "--name".into(),
            "SilentTagged".into(),
            "--features".into(),
            "audio".into(),
            "--areas".into(),
            "countess".into(),
            "--track".into(),
            "none".into(),
            "--events".into(),
        ],
        &root.join("build.jsonl"),
    )
    .unwrap();
    let report: GeneratorReport = serde_json::from_value(value).unwrap();
    validate_generator_output(&root, "SilentTagged", &report, features(1), &[]).unwrap();
    assert_eq!(std::fs::read(excel.join("sounds.txt")).unwrap(), sounds);
    let output = std::fs::read_to_string(
        root.join("SilentTagged/SilentTagged.mpq/data/global/excel/sounds.txt"),
    )
    .unwrap();
    let mut lines = output.lines();
    let columns: Vec<_> = lines.next().unwrap().split('\t').collect();
    let filename = columns.iter().position(|&s| s == "FileName").unwrap();
    for row in lines {
        let cells: Vec<_> = row.split('\t').collect();
        let file = cells.get(filename).unwrap_or(&"");
        assert!(
            file.is_empty() || file.starts_with("audio_telemetry\\"),
            "Unexpected restored audio: {file}"
        );
    }
    println!("Verified empty sound references: {}", root.display());
}

#[test]
#[ignore = "requires original game resources and a built processor"]
fn bundled_processor_clean_rebuild_and_native_join() {
    let game = std::env::var("D2RHUB_LIGHTWEIGHT_GAME_ROOT").unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../artifacts")
        .join(format!("clean-rebuild-{}", uuid::Uuid::new_v4()));
    let base = root.join("Base");
    let mpq = base.join("Base.mpq");
    std::fs::create_dir_all(mpq.join("data/global")).unwrap();
    std::fs::write(
        mpq.join("modinfo.json"),
        r#"{"name":"Base","savepath":"Base/"}"#,
    )
    .unwrap();
    std::fs::write(mpq.join("data/global/dataversionbuild.txt"), "93854").unwrap();
    std::fs::write(mpq.join("data/source-marker.txt"), "original").unwrap();
    let before = snapshot(&base);
    augment(
        &game,
        &root,
        &base,
        "Tools",
        14,
        &[],
        &root.join("first.jsonl"),
    )
    .unwrap();
    assert_eq!(snapshot(&base), before);
    let old = root.join("Tools");
    let layouts = old.join("Tools.mpq/data/global/ui/layouts");
    let hud_path = layouts.join("HudWarningshd.json");
    let hud_bytes = std::fs::read(&hud_path).unwrap();
    let mut hud: serde_json::Value = serde_json::from_slice(&hud_bytes).unwrap();
    assert!(!String::from_utf8_lossy(&hud_bytes)
        .contains("PanelManager:ClosePanel:D2RHubQuickRecreateEscArm"));
    hud["children"].as_array_mut().unwrap().push(serde_json::json!({
        "type": "TimerWidget", "name": "D2RHubCloseEscArm",
        "fields": {"time": 0.001, "message": "PanelManager:ClosePanel:D2RHubQuickRecreateEscArm"}
    }));
    std::fs::write(&hud_path, serde_json::to_vec(&hud).unwrap()).unwrap();
    assert!(validate_audio_mod(&root, "Tools").is_err());
    std::fs::write(&hud_path, hud_bytes).unwrap();
    let join = std::fs::read_to_string(layouts.join("D2RHubInGameJoinGamehd.json")).unwrap();
    assert!(join.contains("JoinGame:JoinGame"));
    assert!(!join.contains("D2RHubCommitJoinGame"));
    assert!(!join.contains("PausePanelMessage:ExitGame"));
    assert!(!layouts.join("D2RHubCommitJoinGamehd.json").exists());
    set_auto_exit_on_death_enabled(&root, "Tools", false).unwrap();
    std::fs::write(old.join("Tools.mpq/data/obsolete-generated.txt"), "stale").unwrap();
    std::fs::write(mpq.join("data/source-marker.txt"), "updated source").unwrap();
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(old.join("d2rhub-mod-manifest.json")).unwrap())
            .unwrap();
    for group in manifest["feature_groups"].as_array_mut().unwrap() {
        if group["id"] == "in_game_room_tools" {
            group["recipe_version"] = 32.into();
            group["fingerprint"] = "room-tools-v32".into();
        }
    }
    for name in ["d2rhub-mod-manifest.json", "audio-telemetry-manifest.json"] {
        std::fs::write(old.join(name), serde_json::to_vec(&manifest).unwrap()).unwrap();
    }
    let (requested, _) = rebuild::recipe(&manifest, features(2)).unwrap();
    assert!(
        requested.room_tools
            && requested.esc_next_game
            && requested.auto_exit_on_death
            && !requested.audio_telemetry
    );
    let transaction = uuid::Uuid::new_v4().simple().to_string();
    let stage = root.join(format!(".d2rhub-upgrade-stage-{transaction}"));
    std::fs::create_dir(&stage).unwrap();
    augment(
        &game,
        &stage,
        &base,
        "Tools",
        14,
        &[],
        &root.join("rebuild.jsonl"),
    )
    .unwrap();
    set_auto_exit_on_death_enabled(&stage, "Tools", false).unwrap();
    replace_audio_mod_directory(
        &root,
        "Tools",
        &stage.join("Tools"),
        &root.join(format!(".d2rhub-upgrade-backup-{transaction}")),
        &[],
    )
    .unwrap();
    assert!(!old.join("Tools.mpq/data/obsolete-generated.txt").exists());
    assert_eq!(
        std::fs::read_to_string(old.join("Tools.mpq/data/source-marker.txt")).unwrap(),
        "updated source"
    );
    let validated = validate_audio_mod_credential(&root, "Tools").unwrap();
    assert!(!validated.auto_exit_on_death_enabled);
    requested
        .validate_present(&validated.feature_groups, PROTOCOL_VERSION)
        .unwrap();
    println!("Verified clean rebuild and native join: {}", root.display());
}

fn features(mask: u8) -> RequestedFeatureGroups {
    RequestedFeatureGroups {
        audio_telemetry: mask & 1 != 0,
        room_tools: mask & 2 != 0,
        esc_next_game: mask & 4 != 0,
        auto_exit_on_death: mask & 8 != 0,
    }
}
fn snapshot(root: &Path) -> BTreeMap<PathBuf, u64> {
    fn visit(root: &Path, path: &Path, entries: &mut BTreeMap<PathBuf, u64>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            assert!(!entry.file_type().unwrap().is_symlink());
            if entry.file_type().unwrap().is_dir() {
                visit(root, &entry.path(), entries);
            } else {
                let mut hash = std::collections::hash_map::DefaultHasher::new();
                std::fs::read(entry.path()).unwrap().hash(&mut hash);
                entries.insert(
                    entry.path().strip_prefix(root).unwrap().to_owned(),
                    hash.finish(),
                );
            }
        }
    }
    let mut entries = BTreeMap::new();
    visit(root, root, &mut entries);
    entries
}
fn invoke(args: &[String], log: &Path) -> Result<serde_json::Value, String> {
    let executable = std::env::var("D2RHUB_MOD_PROCESSOR")
        .expect("Set D2RHUB_MOD_PROCESSOR to the independent processor EXE");
    let output = std::process::Command::new(executable)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    std::fs::write(log, &output.stdout).unwrap();
    std::fs::write(log.with_extension("stderr.log"), &output.stderr).unwrap();
    let events: Vec<serde_json::Value> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    if !output.status.success() {
        return Err(format!(
            "exit {:?}: {} {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr),
            events
                .iter()
                .find(|v| v["type"] == "error")
                .unwrap_or(&serde_json::Value::Null)
        ));
    }
    events
        .into_iter()
        .find(|v| v["type"] == "completed")
        .map(|v| v["report"].clone())
        .ok_or("missing report".into())
}
fn augment(
    game: &str,
    parent: &Path,
    source: &Path,
    name: &str,
    mask: u8,
    required: &[GeneratorFeatureGroup],
    log: &Path,
) -> Result<(), String> {
    let requested = features(mask);
    let args = vec![
        "augment".into(),
        "--game".into(),
        game.into(),
        "--output".into(),
        parent.display().to_string(),
        "--source".into(),
        source.display().to_string(),
        "--name".into(),
        name.into(),
        "--areas".into(),
        "all".into(),
        "--track".into(),
        "all".into(),
        "--features".into(),
        requested.generator_value(),
        "--events".into(),
    ];
    let value = invoke(&args, log)?;
    let report: GeneratorReport = serde_json::from_value(value).map_err(|e| e.to_string())?;
    let validated = validate_generator_output(parent, name, &report, requested, required)?;
    if validated.feature_groups.len() != mask.count_ones() as usize {
        return Err("unexpected feature groups".into());
    }
    let source_name = source.file_name().unwrap().to_string_lossy();
    let version = "data/global/dataversionbuild.txt";
    if std::fs::read(source.join(format!("{source_name}.mpq")).join(version)).unwrap()
        != std::fs::read(parent.join(name).join(format!("{name}.mpq")).join(version))
            .map_err(|e| e.to_string())?
    {
        return Err("game version changed".into());
    }
    if mask & 8 != 0 {
        set_auto_exit_on_death_enabled(parent, name, false)?;
        if validate_audio_mod_credential(parent, name)?.auto_exit_on_death_enabled {
            return Err("death toggle off failed".into());
        }
        set_auto_exit_on_death_enabled(parent, name, true)?;
        if !validate_audio_mod_credential(parent, name)?.auto_exit_on_death_enabled {
            return Err("death toggle on failed".into());
        }
    }
    Ok(())
}

#[test]
#[ignore = "requires downloaded Mod products and original game resources; processes the 3 x 15 feature matrix"]
fn lightweight_all_processing_combinations() {
    let game = std::env::var("D2RHUB_LIGHTWEIGHT_GAME_ROOT").expect("D2RHUB_LIGHTWEIGHT_GAME_ROOT");
    let root = std::env::var("D2RHUB_PROCESSING_TEST_RESUME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../artifacts")
                .join(format!("lightweight-processing-{}", uuid::Uuid::new_v4()))
        });
    std::fs::create_dir_all(&root).unwrap();
    let root = root.canonicalize().unwrap();
    assert_eq!(
        root.parent().unwrap(),
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../artifacts")
            .canonicalize()
            .unwrap()
    );
    assert!(root
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("lightweight-processing-"));
    println!("ARTIFACTS {}", root.display());
    let mut outcomes: Vec<serde_json::Value> = std::fs::read(root.join("results.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    for (profile, base_name) in [("main", "LiteHub"), ("filler", "BoHub"), ("min", "NullHub")] {
        let products = PathBuf::from(std::env::var("D2RHUB_MOD_PRODUCTS_ROOT")
            .expect("Set D2RHUB_MOD_PRODUCTS_ROOT to the directory containing LiteHub, BoHub and NullHub products"));
        let base = products
            .join(base_name)
            .canonicalize()
            .expect("downloaded product directory");
        let identity = crate::lightweight_mod::inspect(&base, base_name)
            .expect("valid product manifest")
            .expect("product identity");
        assert_eq!(identity.profile, profile);
        let original = snapshot(&base);
        for mask in 1..=15u8 {
            let name = format!("{base_name}F{mask:02}");
            if outcomes
                .iter()
                .any(|r| r["case"] == name && r["ok"] == true)
            {
                continue;
            }
            let started = std::time::Instant::now();
            let cached_audio = root.join(format!("{base_name}F01"));
            let source = if mask & 1 != 0 && mask != 1 && mask != 15 {
                &cached_audio
            } else {
                &base
            };
            let result = augment(
                &game,
                &root,
                source,
                &name,
                mask,
                &[],
                &root.join(format!("{name}.jsonl")),
            );
            println!(
                "{name}: {:?} ({:.1}s)",
                result,
                started.elapsed().as_secs_f64()
            );
            outcomes.push(serde_json::json!({"profile":profile,"case":name,"source":source.file_name().unwrap().to_string_lossy(),"features":features(mask).generator_value(),"ok":result.is_ok(),"error":result.err(),"seconds":started.elapsed().as_secs_f64()}));
            std::fs::write(
                root.join("results.json"),
                serde_json::to_vec_pretty(&outcomes).unwrap(),
            )
            .unwrap();
            if outcomes.last().unwrap()["ok"] == true && mask != 1 {
                std::fs::remove_dir_all(root.join(&name)).unwrap();
            }
        }
        let mut source = root.join(format!("{base_name}F01"));
        for (step, mask) in [
            ("addRooms", 3u8),
            ("addEsc", 7),
            ("addDeath", 15),
            ("repeat", 15),
        ] {
            let name = format!("{base_name}_{step}");
            let result =
                validate_audio_mod_credential(&root, source.file_name().unwrap().to_str().unwrap())
                    .and_then(|prior| {
                        augment(
                            &game,
                            &root,
                            &source,
                            &name,
                            mask,
                            &prior.feature_groups,
                            &root.join(format!("{name}.jsonl")),
                        )
                    });
            println!("{name}: {result:?}");
            let success = result.is_ok();
            outcomes.push(serde_json::json!({"profile":profile,"case":name,"ok":success,"error":result.err()}));
            std::fs::write(
                root.join("results.json"),
                serde_json::to_vec_pretty(&outcomes).unwrap(),
            )
            .unwrap();
            if !success {
                break;
            }
            std::fs::remove_dir_all(&source).unwrap();
            source = root.join(name);
        }
        assert_eq!(original, snapshot(&base), "source was mutated");
        // Preserve source and final additive result for manual inspection.
    }
    assert!(
        outcomes.iter().all(|r| r["ok"] == true),
        "See {}",
        root.join("results.json").display()
    );
}

#[test]
#[ignore = "requires completed matrix output; exercises the real same-name replacement transaction"]
fn lightweight_processed_same_name_replacement() {
    let game = std::env::var("D2RHUB_LIGHTWEIGHT_GAME_ROOT").unwrap();
    let root = PathBuf::from(std::env::var("D2RHUB_PROCESSING_TEST_RESUME").unwrap())
        .canonicalize()
        .unwrap();
    assert_eq!(
        root.parent().unwrap(),
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../artifacts")
            .canonicalize()
            .unwrap()
    );
    assert!(root
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("lightweight-processing-"));
    let mut results = Vec::new();
    for base in ["LiteHub", "BoHub", "NullHub"] {
        let name = format!("{base}_repeat");
        let prior = validate_audio_mod_credential(&root, &name).unwrap();
        let transaction = uuid::Uuid::new_v4().simple().to_string();
        let stage = root.join(format!(".d2rhub-upgrade-stage-{transaction}"));
        let backup = root.join(format!(".d2rhub-upgrade-backup-{transaction}"));
        std::fs::create_dir(&stage).unwrap();
        augment(
            &game,
            &stage,
            &root.join(&name),
            &name,
            15,
            &prior.feature_groups,
            &root.join(format!("{base}-same-name.jsonl")),
        )
        .unwrap();
        replace_audio_mod_directory(
            &root,
            &name,
            &stage.join(&name),
            &backup,
            &prior.feature_groups,
        )
        .unwrap();
        let after = validate_audio_mod_credential(&root, &name).unwrap();
        features(15)
            .validate_present(&after.feature_groups, PROTOCOL_VERSION)
            .unwrap();
        recover_audio_mod_replacements(&root).unwrap();
        results
            .push(serde_json::json!({"mod":base,"ok":true,"feature_groups":after.feature_groups}));
        println!("{base} same-name replacement: OK");
        std::fs::write(
            root.join("same-name-results.json"),
            serde_json::to_vec_pretty(&results).unwrap(),
        )
        .unwrap();
    }
}
