//! JS에 노출되는 수집·분석 엔진.
//!
//! 빌드: `wasm-pack build crates/wasm --target web --release`
//!
//! 메인 스레드는 `push()`로 이벤트를 기록만 하고, `tick()`(Worker에서 주기 호출)이
//! 쌓인 이벤트를 특징 추출기로 넘긴 뒤 점수를 갱신한다.
//! 예매 버튼 클릭 시에는 `score()`/`tier()`로 이미 계산된 값만 읽는다.

use guard_collector::{EventKind, EventRecord, RingBuffer};
use guard_features::FeatureExtractor;
use guard_scorer::{Scorer, Tier, Verdict};
use wasm_bindgen::prelude::*;

const RING_CAPACITY: usize = 8192;

#[wasm_bindgen]
pub struct Engine {
    ring: RingBuffer,
    extractor: FeatureExtractor,
    scorer: Scorer,
    verdict: Verdict,
}

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Engine {
        Engine {
            ring: RingBuffer::with_capacity(RING_CAPACITY),
            extractor: FeatureExtractor::new(),
            scorer: Scorer::default(),
            verdict: Verdict {
                score: 50,
                tier: Tier::Observe,
                hard_rule: None,
            },
        }
    }

    /// 이벤트 1건 기록. 알 수 없는 `kind`는 무시한다.
    /// `t_us`는 JS number(f64)로 받는다(u64는 BigInt 변환 비용이 있다).
    pub fn push(&mut self, kind: u8, flags: u8, t_us: f64, x: f32, y: f32, extra: u16) {
        if let Some(kind) = EventKind::from_u8(kind) {
            self.ring
                .push(EventRecord::new(kind, t_us as u64, x, y, flags, extra));
        }
    }

    /// 쌓인 이벤트를 처리하고 점수를 갱신한다. 갱신된 점수를 반환한다.
    pub fn tick(&mut self) -> u8 {
        let extractor = &mut self.extractor;
        self.ring.drain(|e| extractor.update(e));
        self.verdict = self.scorer.score(
            &self.extractor.snapshot(),
            self.extractor.has_sufficient_evidence(),
        );
        self.verdict.score
    }

    pub fn score(&self) -> u8 {
        self.verdict.score
    }

    /// `guard_scorer::Tier` 값 (0=Block, 1=Challenge, 2=Observe, 3=Trusted).
    pub fn tier(&self) -> u8 {
        self.verdict.tier as u8
    }

    /// 서버 재검증용 특징 벡터 (JS에서는 Float32Array).
    pub fn features(&self) -> Vec<f32> {
        self.extractor.snapshot().0.to_vec()
    }

    pub fn dropped_events(&self) -> f64 {
        self.ring.dropped() as f64
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use guard_collector::flags;

    #[test]
    fn injected_input_blocks_after_tick() {
        let mut e = Engine::new();
        e.push(
            EventKind::PointerMove as u8,
            flags::TRUSTED | flags::NATIVE_INJECTED,
            0.0,
            1.0,
            1.0,
            0,
        );
        assert_eq!(e.tier(), Tier::Observe as u8);
        e.tick();
        assert_eq!(e.tier(), Tier::Block as u8);
    }

    #[test]
    fn ignores_unknown_kind() {
        let mut e = Engine::new();
        e.push(200, 0, 0.0, 0.0, 0.0, 0);
        e.tick();
        assert_eq!(e.features()[guard_features::idx::EVENT_COUNT], 0.0);
    }
}
