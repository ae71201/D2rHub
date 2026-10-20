"""Mirror the pinned visual assets to Gitee without publishing a Hub release.

Uses the existing DPAPI publisher credential. Existing attachments are never
overwritten; each public URL is downloaded anonymously and verified in full.
"""
import argparse
import json
import shutil
from pathlib import Path

from release_platforms import CONFIG, Platform, anonymous_verify, credentials, sha256

ROOT = Path(__file__).resolve().parents[1]
TAG = 'vision-resources-v1'
NAMES = {
    'model': 'ch_PP-OCRv5_rec_mobile.onnx',
    'runtime': 'onnxruntime-win-x64-1.22.0.zip',
}


def prepare(cache, output, manifest):
    output.mkdir(parents=True, exist_ok=True)
    files = {}
    for kind, name in NAMES.items():
        asset = manifest[kind]
        source = cache / (asset['sha256'] + ('.onnx' if kind == 'model' else '.zip'))
        if source.stat().st_size != asset['size'] or sha256(source) != asset['sha256']:
            raise RuntimeError(f'{kind}: source size/hash mismatch')
        target = output / name
        shutil.copyfile(source, target)
        files[kind] = target
    return files


def publish(files, manifest, config):
    cfg, token = credentials(config)
    if cfg['gitee_repo'] != 'garyi7e/d2-rhub_-resource':
        raise RuntimeError('Unexpected Gitee resource repository')
    platform = Platform('gitee', cfg['gitee_repo'], token)
    release = platform.release(TAG)
    existing = {asset['name']: asset for asset in platform.assets(release)}
    report = {}
    for kind, path in files.items():
        asset = manifest[kind]
        url = f"https://gitee.com/{cfg['gitee_repo']}/releases/download/{TAG}/{path.name}"
        if path.name not in existing:
            platform.upload(release, path)
        anonymous_verify(url, asset['size'], asset['sha256'])
        report[kind] = {'url': url, 'size': asset['size'], 'sha256': asset['sha256'],
                        'anonymous_verified': True}
        print(f'{kind}: Gitee anonymous size/SHA-256 verification passed', flush=True)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cache', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--config', type=Path, default=CONFIG)
    parser.add_argument('--publish', action='store_true')
    args = parser.parse_args()
    manifest = json.loads((ROOT / 'resources/vision-resources-v1.json').read_text(encoding='utf-8'))
    files = prepare(args.cache, args.output, manifest)
    if args.publish:
        report = publish(files, manifest, args.config)
        (args.output / 'verification.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    else:
        print('Prepared verified visual assets; no uploads performed.')


if __name__ == '__main__':
    main()
