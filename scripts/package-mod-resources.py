"""Build Release assets from existing Mods; never modify the originals."""
import argparse
import hashlib
import json
import pathlib
import shutil
import subprocess
import zipfile

p = argparse.ArgumentParser()
p.add_argument('--mods', type=pathlib.Path, required=True)
p.add_argument('--processor', type=pathlib.Path, required=True)
p.add_argument('--output', type=pathlib.Path, required=True)
p.add_argument('--tag', required=True)
p.add_argument('--revision', type=int, required=True)
a = p.parse_args()
a.output.mkdir(parents=True, exist_ok=True)
version = subprocess.check_output([str(a.processor), '--version'], text=True).split()[1]
assets = []

def record(path, **extra):
    assets.append(dict(url=f'https://github.com/gjy991229/D2rHub/releases/download/{a.tag}/{path.name}',
                       size=path.stat().st_size, sha256=hashlib.file_digest(path.open('rb'), 'sha256').hexdigest()
                       if hasattr(hashlib, 'file_digest') else hashlib.sha256(path.read_bytes()).hexdigest(), **extra))

exe = a.output / f'd2r-audio-mod-{version}-windows-x64.exe'
shutil.copyfile(a.processor, exe)
record(exe, id='processor', version=version, mod_name=None, profile=None, game_data_version=None)
for name, profile in [('LiteHub', 'main'), ('BoHub', 'filler'), ('NullHub', 'min')]:
    root = a.mods / name
    report = json.loads((root / 'generation-manifest.json').read_text(encoding='utf-8-sig'))
    assert report['mod_name'] == name and report['profile'] == profile
    assert report['verified_output_integrity'] is True
    # The source has already been generated. Package its exact files, including any
    # deliberate local refinements; the archive digest authenticates this release.
    report['mod_directory'] = name
    files = sorted(root.rglob('*'))
    assert all(not f.is_symlink() and not (f.stat().st_file_attributes & 0x400)
               for f in files), 'Links/reparse points are not distributable'
    package = a.output / f'{name}-{a.tag}.zip'
    with zipfile.ZipFile(package, 'w', zipfile.ZIP_DEFLATED, compresslevel=6) as z:
        for f in files:
            if f.is_file():
                relative = f.relative_to(root).as_posix()
                if relative == 'generation-manifest.json':
                    z.writestr(f'{name}/{relative}', json.dumps(report, ensure_ascii=False, indent=2))
                else:
                    z.write(f, f'{name}/{relative}')
    record(package, id=name, version=a.tag, mod_name=name, profile=profile,
           game_data_version=report['game_data_version'])
catalog = dict(schema=1, channel='v7-r25-r28-lightweight-v1', revision=a.revision,
               release_url=f'https://github.com/gjy991229/D2rHub/releases/tag/{a.tag}', assets=assets)
(a.output / 'mod-resources-v1.json').write_text(json.dumps(catalog, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
print(json.dumps(catalog, ensure_ascii=False, indent=2))
