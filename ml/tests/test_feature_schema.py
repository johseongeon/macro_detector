import re
import unittest
from pathlib import Path

from guard_ml.feature_schema import FEATURE_NAMES

RUST_FEATURES = Path(__file__).resolve().parents[2] / "crates" / "features" / "src" / "lib.rs"


class FeatureSchemaTest(unittest.TestCase):
    def test_matches_rust_feature_names(self):
        src = RUST_FEATURES.read_text(encoding="utf-8")
        block = re.search(r"FEATURE_NAMES: \[&str; FEATURE_COUNT\] = \[(.*?)\];", src, re.S)
        self.assertIsNotNone(block, "FEATURE_NAMES not found in Rust source")
        rust_names = re.findall(r'"([a-z0-9_]+)"', block.group(1))
        self.assertEqual(rust_names, FEATURE_NAMES)


if __name__ == "__main__":
    unittest.main()
