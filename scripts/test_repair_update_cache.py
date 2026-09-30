"""Verify the one-time repair only moves the known erroneous software cache."""
import copy
import json
from pathlib import Path
import subprocess
import tempfile
import unittest


class CacheRepairTests(unittest.TestCase):
    def repair_command(self, root, running=False):
        # Mock only the process query in this disposable test subprocess.
        # The production script keeps its real running-app protection.
        script = str(Path(__file__).with_name('repair-update-cache.ps1')).replace("'", "''")
        directory = str(root).replace("'", "''")
        process = "[pscustomobject]@{ ProcessName = 'd2rhub' }" if running else '$null'
        wrapper = root / 'invoke-repair.ps1'
        wrapper.write_text(
            'function Get-Process { [CmdletBinding()] param([string]$Name) '
            f'if ($Name -ne \'d2rhub\') {{ throw \'Unexpected process query\' }}; {process} }}\n'
            f"& '{script}' -CacheDirectory '{directory}'\n", encoding='utf-8-sig')
        return ['powershell.exe', '-NoProfile', '-ExecutionPolicy', 'Bypass',
                '-File', str(wrapper)]

    def test_running_hub_blocks_repair_without_touching_cache(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            cache = root / 'software-v2.json'
            cache.write_bytes(b'keep cache')
            result = subprocess.run(self.repair_command(root, running=True), capture_output=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(b'Exit D2RHub completely', result.stderr)
            self.assertEqual(cache.read_bytes(), b'keep cache')
            self.assertEqual(list(root.glob('*.bak')), [])

    def test_repair_is_narrow_reversible_and_idempotent(self):
        known = {'schema': 2, 'kind': 'software', 'product': 'D2RHub',
                 'platform': 'windows-x86_64', 'assets': [{
                     'id': 'hub', 'version': '0.99.106', 'size': 7737897,
                     'sha256': '78e51bb765614c8ad0b92cdb49dd2f9242c828641a5aab9de561c448a55c4332'}]}
        for case in ('known', 'current', 'other-hash', 'malformed', 'missing'):
            with self.subTest(case=case), tempfile.TemporaryDirectory() as folder:
                root = Path(folder)
                cache = root / 'software-v2.json'
                data = copy.deepcopy(known)
                if case == 'current':
                    data['assets'][0]['version'] = '0.9.108'
                if case == 'other-hash':
                    data['assets'][0]['sha256'] = 'a' * 64
                original = b'broken json' if case == 'malformed' else json.dumps(data).encode()
                if case != 'missing':
                    cache.write_bytes(original)
                protected = root / 'resources-v2.json'
                protected.write_bytes(b'keep resources')
                command = self.repair_command(root)
                result = subprocess.run(command, capture_output=True)
                self.assertEqual(result.returncode == 0, case not in ('other-hash', 'malformed'), result.stderr)
                backups = list(root.glob('*.bak'))
                if case == 'known':
                    self.assertFalse(cache.exists())
                    self.assertEqual(len(backups), 1)
                    self.assertEqual(backups[0].read_bytes(), original)
                    self.assertEqual(subprocess.run(command, capture_output=True).returncode, 0)
                    self.assertEqual(len(list(root.glob('*.bak'))), 1)
                elif case != 'missing':
                    self.assertEqual(cache.read_bytes(), original)
                    self.assertEqual(backups, [])
                self.assertEqual(protected.read_bytes(), b'keep resources')


if __name__ == '__main__':
    unittest.main()
