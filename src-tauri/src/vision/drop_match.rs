//! Structured, complete-name matching for modded bilingual item labels.
use super::processing::normalize;
use crate::rune_data::visual_rune_aliases;

pub fn rune(text: &str) -> Option<u32> {
    let text = text.trim().trim_start_matches(['*', '★', '☆', ' ']);
    if text.starts_with('[') || text.starts_with('【') {
        return None;
    }
    let (name, number) = if let Some((name, id)) = text.rsplit_once('#') {
        let id = id
            .trim()
            .parse::<u32>()
            .ok()
            .filter(|v| (1..=33).contains(v))?;
        (name, Some(id))
    } else {
        if text.chars().any(|c| c.is_ascii_digit()) {
            return None;
        }
        (text, None)
    };
    let zh: String = name
        .chars()
        .filter(|c| !c.is_ascii() && c.is_alphabetic())
        .collect();
    let en: String = name.chars().filter(|c| c.is_ascii_alphabetic()).collect();
    if zh.is_empty() && en.is_empty() {
        return None;
    }
    let matches: Vec<_> = (1..=33)
        .filter(|id| {
            let aliases = visual_rune_aliases(*id);
            let accepts = |part: &str| {
                part.is_empty()
                    || aliases
                        .iter()
                        .any(|alias| normalize(alias) == normalize(part))
            };
            accepts(&zh) && accepts(&en) && number.is_none_or(|number| number == *id)
        })
        .collect();
    (matches.len() == 1).then(|| matches[0])
}
pub fn matches(text: &str, aliases: &[String]) -> bool {
    if let Some(number) = rune(text) {
        let names = visual_rune_aliases(number);
        return aliases
            .iter()
            .any(|alias| names.iter().any(|name| normalize(name) == normalize(alias)));
    }
    // A malformed structured rune must never fall back to partial matching.
    if text.contains('#') {
        return false;
    }
    let text = text.trim().trim_start_matches(['*', '★', '☆', ' ']);
    if text.starts_with('[') || text.starts_with('【') {
        return false;
    }
    if aliases
        .iter()
        .any(|alias| !alias.is_empty() && normalize(alias) == normalize(text))
    {
        return true;
    }
    // Complete bilingual equipment names may be concatenated by OCR.
    let zh: String = text
        .chars()
        .filter(|c| !c.is_ascii() && c.is_alphabetic())
        .collect();
    let en: String = text.chars().filter(|c| c.is_ascii_alphabetic()).collect();
    !zh.is_empty()
        && !en.is_empty()
        && [&zh, &en].iter().all(|part| {
            aliases
                .iter()
                .any(|alias| !alias.is_empty() && normalize(alias) == normalize(part))
        })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bilingual_runes_require_agreement_and_ignore_recipes() {
        assert_eq!(rune("*玛尔 Mal#23"), Some(23));
        assert_eq!(rune("普尔Pul#21"), Some(21));
        assert_eq!(rune("海尔 Hel#15"), Some(15));
        assert_eq!(rune("玛尔"), Some(23));
        for text in [
            "玛尔 Pul#23",
            "玛尔 Mal#21",
            "#23",
            "23",
            "[x2+玛尔]",
            "玛尔foo#23",
        ] {
            assert_eq!(rune(text), None, "{text}");
        }
        assert!(matches("*玛尔Mal#23", &visual_rune_aliases(23)));
        assert!(!matches("*玛尔Mal#23", &visual_rune_aliases(21)));
        assert!(!matches("Ber", &["Belt".into()]));
    }
}
