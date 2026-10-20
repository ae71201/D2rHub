import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location(
    "area_aliases", Path(__file__).with_name("generate-area-name-aliases.py")
)
generator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(generator)


class AreaAliasTests(unittest.TestCase):
    def generate(self, rows):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for filename in generator.FILES:
                (root / filename).write_text(json.dumps(rows), encoding="utf-8")
            return generator.generate(root)

    def test_preserves_entry_translation_instead_of_deriving_it_from_title(self):
        result = self.generate([
            {"Key": "Pit Level 1", "enUS": "Pit Level 1", "zhCN": "深坑一层", "koKR": "ignore"},
            {"Key": "EnteringThePitLevel1", "zhCN": "进入深渊一层", "zhTW": "進入地穴第一層", "zhTWO": "进入地穴第一层"},
        ])
        area = result["areas"][0]
        self.assertEqual(area["aliases"]["zhCN"], ["深坑一层", "深渊一层"])
        self.assertEqual(set(area["aliases"]), {"zhCN", "zhTW", "zhTWO"})
        self.assertEqual(area["entry_keys"], ["EnteringThePitLevel1"])
        self.assertEqual(len(result["sources"]), 3)

    def test_removes_rendering_codes_and_keeps_decorated_and_plain_titles(self):
        result = self.generate([
            {"Key": "Sewers Level 1", "enUS": "Lut Gholein Sewers Level 1 ÿc4(13/43/74)", "zhCN": "下水道一层[A2:13/43/74|A3:24/52/85]"},
            {"Key": "EnteringTheSewersLevel1", "zhTWO": "ÿc2【任务标题】\nÿcV进入“鲁·高因下水道 ÿc8第一层ÿcV”ÿc4"},
        ])
        area = result["areas"][0]
        self.assertIn("下水道一层", area["aliases"]["zhCN"])
        self.assertIn("下水道一层[A2:13/43/74|A3:24/52/85]", area["aliases"]["zhCN"])
        self.assertEqual(area["aliases"]["zhTWO"], ["“鲁·高因下水道 第一层”"])
        self.assertIn("Lut Gholein Sewers Level 1", area["names_en"])

    def test_tower_is_a_map_and_to_tower_is_a_portal_label(self):
        result = self.generate([
            {"Key": "Tower Cellar Level 1", "zhCN": "高塔地窖一层"},
            {"Key": "To The Tower Cellar Level 1", "zhCN": "前往高塔地窖一层"},
            {"Key": "EnteringTheTowerCellarLevel1", "zhCN": "进入高塔地牢第一层"},
        ])
        self.assertEqual(len(result["areas"]), 1)
        self.assertEqual(result["areas"][0]["aliases"]["zhCN"], ["高塔地牢第一层", "高塔地窖一层"])

    def test_banner_on_entry_line_does_not_hide_numbered_tomb_translation(self):
        result = self.generate([
            {"Key": "Tal Rasha's Tomb 1", "zhCN": "塔·拉夏的古墓 Ⅻ"},
            {"Key": "EnteringTalRasha'sTomb 1", "zhCN": "ÿc1【ÿcIऄÿc1】ÿcV进入“塔·拉夏的古墓 Ⅻ”"},
        ])
        self.assertIn("“塔·拉夏的古墓 Ⅻ”", result["areas"][0]["aliases"]["zhCN"])

    def test_fails_on_unknown_entry_identity_instead_of_silently_omitting_it(self):
        with self.assertRaisesRegex(ValueError, "Unmapped entry key"):
            self.generate([{"Key": "EnteringUnknownArea", "zhCN": "进入未知地图"}])


if __name__ == "__main__":
    unittest.main()
