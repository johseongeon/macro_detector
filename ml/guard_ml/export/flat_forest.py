"""LightGBM 모델을 `guard_scorer::FlatForest` JSON 형식으로 변환한다.

노드는 전위 순회(pre-order) 순서로 배치한다. 그래서 자식 인덱스가 항상 부모보다 크고,
Rust 쪽 `FlatForest::validate()`가 순회 종료를 보장할 수 있다.
"""

from typing import Any

# crates/scorer/src/gbdt.rs의 LEAF와 같아야 한다.
LEAF = 0xFFFF


def from_lightgbm_dump(dump: dict[str, Any], feature_count: int) -> dict[str, Any]:
    """`Booster.dump_model()` 결과를 변환한다.

    LightGBM은 기본적으로 초기 점수를 첫 트리의 리프에 포함하므로 base_score는 0이다.
    결측값 처리(default_left)는 아직 지원하지 않는다. 클라이언트 특징에는 결측값이 없다.
    """
    out: dict[str, Any] = {
        "base_score": 0.0,
        "feature_count": feature_count,
        "roots": [],
        "feature": [],
        "threshold": [],
        "left": [],
        "right": [],
        "value": [],
    }
    for tree in dump["tree_info"]:
        out["roots"].append(_append_node(out, tree["tree_structure"], feature_count))
    return out


def _append_node(out: dict[str, Any], node: dict[str, Any], feature_count: int) -> int:
    i = len(out["feature"])
    if "leaf_value" in node:
        _push(out, LEAF, 0.0, 0, 0, float(node["leaf_value"]))
        return i

    if node.get("decision_type", "<=") != "<=":
        raise ValueError(f"unsupported decision_type {node['decision_type']!r}")
    feature = int(node["split_feature"])
    if not 0 <= feature < feature_count:
        raise ValueError(f"split_feature {feature} out of range")

    _push(out, feature, float(node["threshold"]), 0, 0, 0.0)
    out["left"][i] = _append_node(out, node["left_child"], feature_count)
    out["right"][i] = _append_node(out, node["right_child"], feature_count)
    return i


def _push(out: dict[str, Any], feature: int, threshold: float, left: int, right: int, value: float) -> None:
    out["feature"].append(feature)
    out["threshold"].append(threshold)
    out["left"].append(left)
    out["right"].append(right)
    out["value"].append(value)


def predict_margin(forest: dict[str, Any], x: list[float]) -> float:
    """Rust 추론기와 동일한 참조 구현. 내보낸 모델의 일치 여부 검증용."""
    total = forest["base_score"]
    for root in forest["roots"]:
        i = root
        while forest["feature"][i] != LEAF:
            f = forest["feature"][i]
            i = forest["left"][i] if x[f] <= forest["threshold"][i] else forest["right"][i]
        total += forest["value"][i]
    return total
