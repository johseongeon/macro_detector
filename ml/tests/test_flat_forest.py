import unittest

from guard_ml.export.flat_forest import LEAF, from_lightgbm_dump, predict_margin

# LightGBM dump_model() 형식의 작은 예제: 트리 2개
DUMP = {
    "tree_info": [
        {
            "tree_structure": {
                "split_feature": 0,
                "threshold": 0.5,
                "decision_type": "<=",
                "left_child": {"leaf_value": -1.0},
                "right_child": {
                    "split_feature": 1,
                    "threshold": 10.0,
                    "decision_type": "<=",
                    "left_child": {"leaf_value": 0.5},
                    "right_child": {"leaf_value": 2.0},
                },
            }
        },
        {"tree_structure": {"leaf_value": 0.25}},
    ]
}


class FlatForestTest(unittest.TestCase):
    def setUp(self):
        self.forest = from_lightgbm_dump(DUMP, feature_count=2)

    def test_children_come_after_parent(self):
        f = self.forest
        for i, feat in enumerate(f["feature"]):
            if feat != LEAF:
                self.assertGreater(f["left"][i], i)
                self.assertGreater(f["right"][i], i)

    def test_predicts_like_original_trees(self):
        self.assertEqual(predict_margin(self.forest, [0.1, 0.0]), -0.75)
        self.assertEqual(predict_margin(self.forest, [0.9, 5.0]), 0.75)
        self.assertEqual(predict_margin(self.forest, [0.9, 50.0]), 2.25)

    def test_rejects_unknown_feature(self):
        with self.assertRaises(ValueError):
            from_lightgbm_dump(DUMP, feature_count=1)


if __name__ == "__main__":
    unittest.main()
