"""Empty BoHub sound references and retain valid native channel configuration.

Makes a timestamped backup of every changed file before installing the edits.
No other Mod or game settings are modified.
"""
import argparse
import csv
from datetime import datetime
import hashlib
import io
import json
from pathlib import Path
import shutil


def silent_sounds(data):
    text = data.decode('utf-8-sig')
    reader = csv.DictReader(io.StringIO(text), delimiter='\t')
    fields = reader.fieldnames
    if not fields or not {'Sound', 'FileName', 'Redirect', 'Volume Min', 'Volume Max'} <= set(fields):
        raise ValueError('Unexpected sounds.txt schema')
    rows = list(reader)
    for row in rows:
        if None in row:
            raise ValueError('Malformed sound row')
        row.update(FileName='', Redirect='', **{'Volume Min': '0', 'Volume Max': '0'})
    output = io.StringIO(newline='')
    writer = csv.DictWriter(output, fields, delimiter='\t', lineterminator='\r\n')
    writer.writeheader()
    writer.writerows(rows)
    return output.getvalue().encode('utf-8'), len(rows)


def apply(mod, sounds, native):
    mod = mod.resolve(strict=True)
    if mod.name != 'BoHub' or not (mod / 'BoHub.mpq').is_dir():
        raise ValueError('Expected an unpacked BoHub directory')
    if mod.is_symlink() or (mod / 'BoHub.mpq').is_symlink():
        raise ValueError('Mod directory must not be a link')
    sound_data, count = silent_sounds(sounds.read_bytes())
    channels = json.loads((native / 'soundchannels.json').read_text(encoding='utf-8-sig'))
    if not channels.get('channels'):
        raise ValueError('Missing native sound channels')
    settings = json.loads((native / 'soundsettings.json').read_text(encoding='utf-8-sig'))
    encode = lambda value: (json.dumps(value, ensure_ascii=False, indent=2) + '\n').encode('utf-8')
    changes = {
        'BoHub.mpq/data/global/excel/sounds.txt': sound_data,
        'BoHub.mpq/data/hd/global/excel/soundchannels.json': encode(channels),
        'BoHub.mpq/data/hd/global/excel/soundsettings.json': encode(settings),
    }
    metadata_path = mod / 'mod-version.json'
    next_version = datetime.now().strftime('%Y.%m.%d') + '.1'
    if metadata_path.exists():
        metadata = json.loads(metadata_path.read_text(encoding='utf-8-sig'))
        metadata['mod_version'] = next_version
        metadata['published'] = False
        changes['mod-version.json'] = encode(metadata)
    manifest_path = mod / 'enhancement-manifest.json'
    if manifest_path.exists():
        manifest = json.loads(manifest_path.read_text(encoding='utf-8-sig'))
        manifest['mod_version'] = next_version
        manifest.setdefault('features', {})['silent_audio'] = {
            'sound_rows': count, 'file_and_redirect_references': 'empty',
            'channels': 'native_routing', 'runtime_verified': False,
        }
        for name, data in changes.items():
            manifest.setdefault('files', {})[name.removeprefix('BoHub.mpq/')] = {
                'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest(),
            }
        manifest['files'].pop('mod-version.json', None)
        manifest['modified_paths'] = sorted(set(manifest.get('modified_paths', [])) | {
            name.removeprefix('BoHub.mpq/') for name in changes if name.startswith('BoHub.mpq/')
        })
        changes['enhancement-manifest.json'] = encode(manifest)
    backup = mod.parent / '0备份' / ('BoHub-before-silence-' + datetime.now().strftime('%Y%m%d-%H%M%S-%f'))
    # Confirm every resolved target remains in BoHub before any writes.
    targets = {name: mod / name for name in changes}
    for target in targets.values():
        target.resolve().relative_to(mod)
    backup.mkdir(parents=True, exist_ok=False)
    originals = {}
    for name, target in targets.items():
        originals[name] = target.exists()
        if target.exists():
            saved = backup / name
            saved.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(target, saved)
    (backup / 'restore-manifest.json').write_bytes(encode(originals))
    try:
        for name, data in changes.items():
            target = targets[name]
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
        parsed = list(csv.DictReader(io.StringIO(targets['BoHub.mpq/data/global/excel/sounds.txt'].read_text(encoding='utf-8')), delimiter='\t'))
        assert len(parsed) == count and all(not row['FileName'] and not row['Redirect'] for row in parsed)
        assert json.loads(targets['BoHub.mpq/data/hd/global/excel/soundchannels.json'].read_text())['channels'] == channels['channels']
    except Exception:
        for name, existed in originals.items():
            if existed:
                shutil.copy2(backup / name, targets[name])
            else:
                targets[name].unlink(missing_ok=True)
        raise
    return {'sound_rows': count, 'backup': str(backup), 'mod': str(mod)}


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--mod', required=True, type=Path)
    parser.add_argument('--sounds', required=True, type=Path)
    parser.add_argument('--native-audio', required=True, type=Path)
    args = parser.parse_args()
    print(json.dumps(apply(args.mod, args.sounds, args.native_audio), ensure_ascii=False))
