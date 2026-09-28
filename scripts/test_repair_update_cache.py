"""Verify the one-time repair only moves the known erroneous software cache."""
import copy
import json
from pathlib import Path
import subprocess
import tempfile
import unittest


class CacheRepairTests(unittest.TestCase):
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
                command = ['powershell.exe', '-NoProfile', '-ExecutionPolicy', 'Bypass',
                           '-File', str(Path(__file__).with_name('repair-update-cache.ps1')),
                           '-CacheDirectory', folder]
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
