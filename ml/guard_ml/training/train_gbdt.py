"""Stage 2 GBDT 학습.

사용법: python -m guard_ml.training.train_gbdt --train data.npz --out artifacts/

TODO(Phase 2):
- LightGBM 학습 (트리 200개 이하, 깊이 제한으로 클라이언트 추론 1ms 이내 유지)
- 검증셋에서 isotonic 보정 → Calibrator(xs, ys) 저장
- 목표 FPR 기준 임계값 산출 (guard_ml.eval.metrics.threshold_for_fpr)
"""

import argparse
import json
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--train", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--num-trees", type=int, default=200)
    args = parser.parse_args()

    import lightgbm as lgb  # 무거운 의존성은 실제 학습 시에만 로드
    import numpy as np

    from guard_ml.export.flat_forest import from_lightgbm_dump
    from guard_ml.feature_schema import FEATURE_COUNT, FEATURE_NAMES

    data = np.load(args.train)
    x, y = data["x"], data["y"]  # y: 1 = 매크로, 0 = 사람
    if x.shape[1] != FEATURE_COUNT:
        raise SystemExit(f"expected {FEATURE_COUNT} features, got {x.shape[1]}")

    booster = lgb.train(
        {"objective": "binary", "num_leaves": 15, "learning_rate": 0.05, "verbose": -1},
        lgb.Dataset(x, y, feature_name=FEATURE_NAMES),
        num_boost_round=args.num_trees,
    )

    args.out.mkdir(parents=True, exist_ok=True)
    forest = from_lightgbm_dump(booster.dump_model(), FEATURE_COUNT)
    (args.out / "forest.json").write_text(json.dumps(forest))


if __name__ == "__main__":
    main()
