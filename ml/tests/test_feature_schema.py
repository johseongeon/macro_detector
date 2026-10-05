import re
import unittest
from pathlib import Path

from guard_ml.feature_schema import FEATURE_NAMES

REPO = Path(__file__).resolve().parents[2]
RUST_FEATURES = REPO / "crates" / "features" / "src" / "lib.rs"
JS_BOOTSTRAP = REPO / "apps" / "browser" / "bootstrap" / "collector-bootstrap.js"


def names_in(path: Path, pattern: str) -> list[str]:
    block = re.search(pattern, path.read_text(encoding="utf-8"), re.S)
    if block is None:
        raise AssertionError(f"feature name list not found in {path}")
    return re.findall(r'"([a-z0-9_]+)"', block.group(1))


class FeatureSchemaTest(unittest.TestCase):
    def test_matches_rust_feature_names(self):
        rust = names_in(RUST_FEATURES, r"FEATURE_NAMES: \[&str; FEATURE_COUNT\] = \[(.*?)\];")
        self.assertEqual(rust, FEATURE_NAMES)

    def test_matches_bootstrap_feature_names(self):
        js = names_in(JS_BOOTSTRAP, r"const FEATURE_NAMES = \[(.*?)\];")
        self.assertEqual(js, FEATURE_NAMES)


if __name__ == "__main__":
    unittest.main()
