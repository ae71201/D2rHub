"""Exercise publication adapters without credentials or network access."""
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

from release_platforms import Platform, SourceCommitMismatch


class ReleaseAssetsTests(unittest.TestCase):
    def test_software_release_targets_prepared_sha_not_main(self):
        platform = Platform('github', 'owner/repo', '')
        source = 'a' * 40
        release = {'id': 7, 'tag_name': 'v1.0.0'}
        with patch.object(platform, 'call', side_effect=[{'sha': source}, None, None, release]) as call:
            self.assertEqual(platform.release('v1.0.0', source_commit=source), release)
        created = call.call_args.kwargs['json']
        self.assertEqual(created['target_commitish'], source)
        self.assertIn(source, created['body'])
        self.assertEqual(call.call_args_list[1].args[1], '/git/ref/tags/v1.0.0')

    def test_existing_wrong_tag_rejected_even_without_release(self):
        platform = Platform('github', 'owner/repo', '')
        source = 'a' * 40
        with patch.object(platform, 'call', side_effect=[{'sha': source}, {'ref': 'refs/tags/v1.0.0'}, {'sha': 'b' * 40}]) as call:
            with self.assertRaises(SourceCommitMismatch):
                platform.release('v1.0.0', source_commit=source)
        self.assertTrue(all(c.args[0] == 'GET' for c in call.call_args_list))

    def test_unpushed_source_rejected_without_mutation(self):
        platform = Platform('github', 'owner/repo', '')
        with patch.object(platform, 'call', return_value=None) as call:
            with self.assertRaises(SourceCommitMismatch):
                platform.release('v1.0.0', source_commit='a' * 40)
        self.assertEqual(call.call_count, 1)

    def test_retry_accepts_matching_existing_tag(self):
        platform = Platform('github', 'owner/repo', '')
        source = 'a' * 40
        release = {'id': 7, 'tag_name': 'v1.0.0'}
        with patch.object(platform, 'call', side_effect=[{'sha': source}, {'ref': 'refs/tags/v1.0.0'}, {'sha': source}, release]) as call:
            self.assertEqual(platform.release('v1.0.0', source_commit=source), release)
        self.assertTrue(all(c.args[0] == 'GET' for c in call.call_args_list))

    def test_lists_later_pages_on_both_platforms(self):
        for name, collection in [('github', 'assets'), ('gitee', 'attach_files')]:
            with self.subTest(platform=name):
                platform = Platform(name, 'owner/repo', '')
                first = [{'id': i, 'name': f'snapshot-{i}.json'} for i in range(100)]
                last = {'id': 100, 'name': 'snapshot-100.json'}
                with patch.object(platform, 'call', side_effect=[first, [last]]) as call:
                    self.assertEqual(platform.assets({'id': 7}), first + [last])
                self.assertEqual([c.args[1] for c in call.call_args_list], [
                    f'/releases/7/{collection}?page=1&per_page=100',
                    f'/releases/7/{collection}?page=2&per_page=100',
                ])

    def test_exact_full_page_stops_on_empty_next_page(self):
        platform = Platform('github', 'owner/repo', '')
        page = [{'id': i} for i in range(100)]
        with patch.object(platform, 'call', side_effect=[page, []]) as call:
            self.assertEqual(platform.assets({'id': 7}), page)
            self.assertEqual(call.call_count, 2)

    def test_upload_finds_new_snapshot_after_first_hundred_assets(self):
        platform = Platform('github', 'owner/repo', '')
        old = [{'name': f'snapshot-{i}.json'} for i in range(100)]
        uploaded = {'name': 'new-snapshot.json', 'browser_download_url': 'https://example.invalid/new'}
        with patch('release_platforms.subprocess.run', return_value=subprocess.CompletedProcess([], 0)), \
                patch.object(platform, 'call', side_effect=[old, [uploaded]]):
            self.assertEqual(platform.upload({'id': 7, 'tag_name': 'index'}, Path('new-snapshot.json')), uploaded)


if __name__ == '__main__':
    unittest.main()
