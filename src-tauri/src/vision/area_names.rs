//! Exact map and entry-message translations extracted from the three supplied
//! levels JSONs. English names/string keys identify a map; Chinese is evidence,
//! never a fuzzy identity lookup. Identical displayed names share a scene;
//! aliases pointing to different named scenes remain ambiguous.
use super::processing;
use serde::Deserialize;
use std::{collections::HashMap, sync::OnceLock};

#[derive(Deserialize)]
struct TranslationFile {
    areas: Vec<Translation>,
}

#[derive(Deserialize)]
struct Translation {
    names_en: Vec<String>,
    aliases: HashMap<String, Vec<String>>,
}

fn identity(name: &str) -> String {
    let name = name.trim().to_ascii_lowercase();
    processing::normalize(name.strip_prefix("the ").unwrap_or(&name))
}

pub(super) fn aliases(name_en: &str) -> Vec<String> {
    static INDEX: OnceLock<HashMap<String, Vec<String>>> = OnceLock::new();
    INDEX
        .get_or_init(|| {
            let file: TranslationFile =
                serde_json::from_str(include_str!("../../../resources/area-name-aliases.json"))
                    .expect("embedded area translations must be valid");
            let mut index: HashMap<String, Vec<String>> = HashMap::new();
            for area in file.areas {
                let aliases: Vec<_> = ["zhCN", "zhTW", "zhTWO"]
                    .into_iter()
                    .filter_map(|locale| area.aliases.get(locale))
                    .flatten()
                    .map(|name| processing::normalize(name))
                    .filter(|name| !name.is_empty())
                    .collect();
                for name in area.names_en {
                    index
                        .entry(identity(&name))
                        .or_default()
                        .extend(aliases.clone());
                }
            }
            for aliases in index.values_mut() {
                aliases.sort();
                aliases.dedup();
            }
            index
        })
        .get(&identity(name_en))
        .cloned()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_both_map_names_and_distinct_entry_translations() {
        for (english, expected) in [
            (
                "Rogue Encampment",
                vec!["流亡者营地", "罗格营地", "萝格营地", "俠盜營地", "侠盗营地"],
            ),
            (
                "Pit Level 1",
                vec!["深坑一层", "深渊一层", "地穴第一层", "地穴第一層"],
            ),
            (
                "Pit Level 2",
                vec!["深坑二层", "深渊二层", "地穴第二层", "地穴第二層"],
            ),
            ("Moo Moo Farm", vec!["哞哞农场", "哞哞農莊", "秘密母牛关卡"]),
            ("Outer Cloister", vec!["外侧回廊", "外側迴廊"]),
        ] {
            let names = aliases(english);
            for name in expected {
                assert!(
                    names.contains(&processing::normalize(name)),
                    "{english}: {name}"
                );
            }
        }
        assert_eq!(aliases("Chaos Sanctum"), aliases("The Chaos Sanctuary"));
        assert_eq!(
            aliases("Glacial Caves Level 1"),
            aliases("The Ancients' Way")
        );
        assert!(aliases("Unlisted Mod Area").is_empty());
        assert!(!aliases("Pit Level 1").contains(&processing::normalize("深渊立层")));
        assert!(!aliases("Outer Cloister").contains(&processing::normalize("外廊")));
    }
}
