"""특징 벡터 스키마.

`crates/features/src/lib.rs`의 `FEATURE_NAMES`와 순서까지 정확히 일치해야 한다.
한쪽만 바꾸면 클라이언트 추론 결과가 조용히 틀어지므로, 테스트에서 두 목록을 비교한다.
"""

FEATURE_NAMES: list[str] = [
    "event_count",
    "move_count",
    "speed_mean",
    "speed_std",
    "click_interval_mean_ms",
    "click_interval_std_ms",
    "key_dwell_mean_ms",
    "key_dwell_std_ms",
    "untrusted_ratio",
    "injected_count",
    "native_mismatch_count",
]

FEATURE_COUNT = len(FEATURE_NAMES)
