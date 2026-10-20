"""Visual mirror publication must verify bytes and remain separate from Hub releases."""
import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import Mock, patch

spec = importlib.util.spec_from_file_location('vision_publisher', Path(__file__).with_name('publish-vision-resources.py'))
publisher = importlib.util.module_from_spec(spec)
spec.loader.exec_module(publisher)


class VisionPublisherTests(unittest.TestCase):
    def manifest(self):
        return {kind: {'size': len(data), 'sha256': hashlib.sha256(data).hexdigest()}
                for kind, data in [('model', b'model'), ('runtime', b'runtime')]}

    def test_prepare_rejects_corrupt_source_before_publishing(self):
        with tempfile.TemporaryDirectory() as temp:
            cache = Path(temp) / 'cache'
            cache.mkdir()
            manifest = self.manifest()
            (cache / (manifest['model']['sha256'] + '.onnx')).write_bytes(b'wrong')
            with self.assertRaisesRegex(RuntimeError, 'source size/hash mismatch'):
                publisher.prepare(cache, Path(temp) / 'output', manifest)

    def test_existing_attachments_are_verified_without_uploading_or_touching_software(self):
        files = {kind: Path(name) for kind, name in publisher.NAMES.items()}
        platform = Mock()
        platform.release.return_value = {'id': 1}
        platform.assets.return_value = [{'name': path.name} for path in files.values()]
        with (patch.object(publisher, 'credentials', return_value=({'gitee_repo': 'garyi7e/d2-rhub_-resource'}, 'private')),
             patch.object(publisher, 'Platform', return_value=platform),
             patch.object(publisher, 'anonymous_verify', return_value=True) as verify):
            report = publisher.publish(files, self.manifest(), Path('unused'))
        platform.release.assert_called_once_with('vision-resources-v1')
        platform.upload.assert_not_called()
        self.assertEqual(verify.call_count, 2)
        self.assertTrue(all(asset['anonymous_verified'] for asset in report.values()))

    def test_anonymous_download_failure_does_not_report_success(self):
        platform = Mock()
        platform.assets.return_value = []
        with (patch.object(publisher, 'credentials', return_value=({'gitee_repo': 'garyi7e/d2-rhub_-resource'}, 'private')),
             patch.object(publisher, 'Platform', return_value=platform),
             patch.object(publisher, 'anonymous_verify', side_effect=RuntimeError('hash mismatch'))):
            with self.assertRaisesRegex(RuntimeError, 'hash mismatch'):
                publisher.publish({'model': Path(publisher.NAMES['model'])}, self.manifest(), Path('unused'))


if __name__ == '__main__':
    unittest.main()
