"""Retired v1 packaging entry point; keep a clear migration error for old callers."""
import sys

if __name__ == '__main__':
    print(
        'The legacy v1 packager is retired. Use .\\release.ps1 -Target mods '
        'or npm run package:mod-resources. Configure source paths in the release '
        'workflow; see docs/release-workflow.md. No files were changed.',
        file=sys.stderr,
    )
    raise SystemExit(2)
