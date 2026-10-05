import unittest

from guard_ml.eval.metrics import fpr_by_segment, rate_at_or_above, threshold_for_fpr, tpr_at_fpr


class MetricsTest(unittest.TestCase):
    def test_threshold_meets_target_fpr(self):
        humans = [i / 100 for i in range(100)]  # 0.00 ~ 0.99
        thr = threshold_for_fpr(humans, 0.05)
        self.assertLessEqual(rate_at_or_above(humans, thr), 0.05)
        self.assertAlmostEqual(thr, 0.95)

    def test_ties_do_not_exceed_target(self):
        humans = [0.1] * 90 + [0.9] * 10
        thr = threshold_for_fpr(humans, 0.05)
        self.assertEqual(rate_at_or_above(humans, thr), 0.0)
        self.assertGreater(thr, 0.9)

    def test_tpr_at_fpr(self):
        thr, tpr = tpr_at_fpr([0.1, 0.2, 0.3, 0.4], [0.35, 0.5, 0.9, 0.05], 0.25)
        self.assertEqual(thr, 0.4)
        self.assertEqual(tpr, 0.5)

    def test_fpr_by_segment(self):
        result = fpr_by_segment({"touchpad": [0.1, 0.8], "keyboard_only": [0.1, 0.2]}, 0.5)
        self.assertEqual(result, {"touchpad": 0.5, "keyboard_only": 0.0})


if __name__ == "__main__":
    unittest.main()
