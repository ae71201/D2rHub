//! Opt-in integration matrix: real bundled sidecar, original game, Hub validators.
use super::*;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

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
    let executable = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("binaries/d2r-audio-mod-x86_64-pc-windows-msvc.exe");
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
#[ignore = "requires original game resources; generates the 3 x 15 feature matrix"]
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
        let base = root.join(base_name);
        if !base.exists() {
            invoke(
                &[
                    "lightweight".into(),
                    "--game".into(),
                    game.clone(),
                    "--profile".into(),
                    profile.into(),
                    "--name".into(),
                    base_name.into(),
                    "--output".into(),
                    root.display().to_string(),
                    "--events".into(),
                ],
                &root.join(format!("{base_name}-base.jsonl")),
            )
            .unwrap();
        }
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
            .validate_present(&after.feature_groups)
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
