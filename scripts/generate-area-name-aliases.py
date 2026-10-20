"""Extract exact Chinese map/entry translations from the supplied levels JSONs.

Usage: python scripts/generate-area-name-aliases.py <resources-directory>
The source files stay outside the repository; their hashes are saved in the output.
"""

import argparse
import hashlib
import json
import re
from pathlib import Path

FILES = ("levels-ch.json", "levels-chtw.json", "levels-mdk.json")
LOCALES = ("zhCN", "zhTW", "zhTWO")
# These announcement keys do not share their map key's spelling. In particular,
# the post-quest Den has the same visible name as Cave Level 1: keep that ambiguity.
ENTRY_KEYS = {
    "EnteringTheDenOfEvilInQuest": "Den of Evil",
    "EnteringTheDenOfEvilPostQuest": "Den of Evil",
    "EnteringTheGlacierCavesLevel1": "Glacial Caves Level 1",
    "EnteringTheGlacierCavesLevel2": "Glacial Caves Level 2",
    "EnteringThePandemoniumFinale": "DiabloTristram",
}


def identity(text):
    return re.sub(r"[^a-z0-9]", "", re.sub(r"^the\s*", "", text, flags=re.I).lower())


def visible_line(text, entry=False):
    text = re.sub(r"ÿc.", "", text)
    # A banner may precede the announcement on the same line (the seven tombs).
    text = re.sub(r"【[^】]*】", "", text)
    lines = [line.strip() for line in text.splitlines() if line.strip()]
    if entry:
        lines = [line for line in lines if line.startswith(("进入", "進入", "Entering "))]
    if not lines:
        return ""
    return lines[0]


def without_levels(text):
    # Only numeric level annotations, including [A2:...|A3:...], are removed.
    text = re.sub(r"\[(?:A[1-5]:)?\d[\d/A:|\s]*\]", "", text)
    return re.sub(r"\(\d[\d/\s]*\)", "", text).strip()


def generate(directory):
    sources = []
    rows = []
    for name in FILES:
        data = (directory / name).read_bytes()
        sources.append({"file": name, "sha256": hashlib.sha256(data).hexdigest()})
        rows.extend(json.loads(data.decode("utf-8-sig")))
    bases = {
        row["Key"] for row in rows
        if not row["Key"].startswith("Entering")
        and not re.match(r"^To(?: |[A-Z])", row["Key"])
        and not re.fullmatch(r"act\dX?|Leave|LevelsAddNewKeys|MooMooFarmGroup", row["Key"])
    }
    by_key = {identity(key): key for key in bases}
    entries = {
        key: {"key": key, "entry_keys": set(), "names_en": {key},
              "aliases": {locale: set() for locale in LOCALES}}
        for key in sorted(bases)
    }
    for row in rows:
        key = row["Key"]
        entry = key.startswith("Entering")
        if entry:
            target = ENTRY_KEYS.get(key) or by_key.get(identity(key[len("Entering"):]))
            if target not in entries:
                raise ValueError(f"Unmapped entry key: {key}")
        elif key in entries:
            target = key
        else:
            continue
        record = entries[target]
        if entry:
            record["entry_keys"].add(key)
        else:
            english = without_levels(visible_line(row.get("enUS", "")))
            if english:
                record["names_en"].add(english)
        for locale in LOCALES:
            line = visible_line(row.get(locale, ""), entry)
            if entry and row.get(locale, "").strip() and not line:
                raise ValueError(f"Unreadable entry translation: {key}/{locale}")
            if entry:
                line = re.sub(r"^(?:进入|進入)\s*", "", line)
            if line:
                record["aliases"][locale].update((line, without_levels(line)))
    for record in entries.values():
        record["entry_keys"] = sorted(record["entry_keys"])
        record["names_en"] = sorted(record["names_en"])
        record["aliases"] = {locale: sorted(names) for locale, names in record["aliases"].items()}
    return {"sources": sources, "locales": list(LOCALES), "areas": list(entries.values())}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--output", type=Path, default=Path(__file__).resolve().parents[1] / "resources/area-name-aliases.json")
    args = parser.parse_args()
    result = generate(args.directory)
    args.output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"Generated {len(result['areas'])} area translation groups: {args.output}")
