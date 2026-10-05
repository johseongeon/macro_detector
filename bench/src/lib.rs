//! 벤치마크용 합성 세션 생성기.

use guard_collector::{flags, EventKind, EventRecord};

/// 8ms 간격(125Hz)으로 원을 그리는 마우스 이동 + 주기적 클릭으로 이루어진 세션.
pub fn synthetic_session(n: usize) -> Vec<EventRecord> {
    (0..n)
        .map(|i| {
            let t_us = i as u64 * 8_000;
            let a = i as f32 * 0.05;
            let kind = if i % 50 == 49 {
                EventKind::PointerDown
            } else {
                EventKind::PointerMove
            };
            EventRecord::new(
                kind,
                t_us,
                500.0 + 200.0 * a.cos(),
                400.0 + 200.0 * a.sin(),
                flags::TRUSTED,
                0,
            )
        })
        .collect()
}
