"""오판 최소화 중심 평가 지표.

점수 규약: `p_bot`(매크로일 확률)이 높을수록 매크로. `p_bot >= threshold`이면 매크로로 판정.
"""

import bisect
import math
from collections.abc import Sequence


def threshold_for_fpr(human_scores: Sequence[float], target_fpr: float) -> float:
    """사람 표본의 FPR이 `target_fpr` 이하가 되는 가장 낮은 임계값.

    조건을 만족하는 값이 사람 점수 중에 없으면 최댓값보다 큰 값을 반환한다(아무도 차단하지 않음).
    """
    if not human_scores:
        raise ValueError("human_scores must not be empty")
    if not 0.0 <= target_fpr <= 1.0:
        raise ValueError("target_fpr must be within [0, 1]")

    ordered = sorted(human_scores)
    n = len(ordered)
    for thr in sorted(set(ordered)):
        false_positives = n - bisect.bisect_left(ordered, thr)
        if false_positives / n <= target_fpr:
            return thr
    return math.nextafter(ordered[-1], math.inf)


def rate_at_or_above(scores: Sequence[float], threshold: float) -> float:
    """임계값 이상으로 판정된 비율. 사람 표본이면 FPR, 매크로 표본이면 TPR."""
    if not scores:
        return 0.0
    return sum(1 for s in scores if s >= threshold) / len(scores)


def tpr_at_fpr(human_scores: Sequence[float], bot_scores: Sequence[float], target_fpr: float) -> tuple[float, float]:
    """(임계값, 해당 임계값에서의 TPR)."""
    thr = threshold_for_fpr(human_scores, target_fpr)
    return thr, rate_at_or_above(bot_scores, thr)


def fpr_by_segment(segments: dict[str, Sequence[float]], threshold: float) -> dict[str, float]:
    """집단별(터치패드, 키보드 전용, 고령자 등) FPR. 집단마다 상한을 따로 검사한다."""
    return {name: rate_at_or_above(scores, threshold) for name, scores in segments.items()}
