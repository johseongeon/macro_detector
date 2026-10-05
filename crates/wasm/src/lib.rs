//! 페이지에 주입되는 수집·분석 엔진.
//!
//! 빌드: `cargo build -p guard-wasm --target wasm32-unknown-unknown --release`
//! (브라우저 셸의 `build.rs`가 자동으로 빌드해 exe에 내장한다.)
//!
//! JS 글루 코드 없이 `WebAssembly.instantiate(bytes, {})`만으로 쓸 수 있도록
//! import가 없는 C ABI 함수(`guard_*`)를 내보낸다. 엔진은 모듈 인스턴스당 하나다.
//!
//! 메인 스레드는 `guard_push()`로 이벤트를 기록만 하고, `guard_tick()`(주기 호출)이
//! 쌓인 이벤트를 특징 추출기로 넘긴 뒤 점수를 갱신한다.
//! 예매 버튼 클릭 시에는 `guard_score()`/`guard_tier()`로 이미 계산된 값만 읽는다.

use std::cell::RefCell;

use guard_collector::{EventKind, EventRecord, RingBuffer};
use guard_features::{FeatureExtractor, FeatureVector, FEATURE_COUNT};
use guard_scorer::{Scorer, Tier, Verdict};

const RING_CAPACITY: usize = 8192;

pub struct Engine {
    ring: RingBuffer,
    extractor: FeatureExtractor,
    scorer: Scorer,
    verdict: Verdict,
    features: FeatureVector,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            ring: RingBuffer::with_capacity(RING_CAPACITY),
            extractor: FeatureExtractor::new(),
            scorer: Scorer::default(),
            verdict: Verdict {
                score: 50,
                tier: Tier::Observe,
                hard_rule: None,
            },
            features: FeatureVector([0.0; FEATURE_COUNT]),
        }
    }

    /// 이벤트 1건 기록. 알 수 없는 `kind`는 무시한다.
    pub fn push(&mut self, kind: u8, flags: u8, t_us: u64, x: f32, y: f32, extra: u16) {
        if let Some(kind) = EventKind::from_u8(kind) {
            self.ring
                .push(EventRecord::new(kind, t_us, x, y, flags, extra));
        }
    }

    /// 쌓인 이벤트를 처리하고 점수를 갱신한다.
    pub fn tick(&mut self) -> Verdict {
        let extractor = &mut self.extractor;
        self.ring.drain(|e| extractor.update(e));
        self.features = self.extractor.snapshot();
        self.verdict = self
            .scorer
            .score(&self.features, self.extractor.has_sufficient_evidence());
        self.verdict
    }

    pub fn verdict(&self) -> Verdict {
        self.verdict
    }

    /// 마지막 `tick()` 시점의 특징 벡터.
    pub fn features(&self) -> &FeatureVector {
        &self.features
    }

    pub fn has_sufficient_evidence(&self) -> bool {
        self.extractor.has_sufficient_evidence()
    }

    pub fn dropped_events(&self) -> u64 {
        self.ring.dropped()
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

thread_local! {
    // wasm32-unknown-unknown은 단일 스레드이므로 사실상 전역 엔진이다.
    static ENGINE: RefCell<Engine> = RefCell::new(Engine::new());
}

fn with_engine<T>(f: impl FnOnce(&mut Engine) -> T) -> T {
    ENGINE.with(|e| f(&mut e.borrow_mut()))
}

// ---- JS에 노출되는 C ABI ----
// 이름과 시그니처는 apps/browser/bootstrap/collector-bootstrap.js와 일치해야 한다.
// wasm에서 u32는 i32, f64는 JS number로 전달된다(u64는 BigInt 변환 비용이 있어 쓰지 않는다).

#[no_mangle]
pub extern "C" fn guard_push(kind: u32, flags: u32, t_us: f64, x: f32, y: f32, extra: u32) {
    with_engine(|e| e.push(kind as u8, flags as u8, t_us as u64, x, y, extra as u16));
}

/// 점수를 갱신하고 반환한다 (0~100, 높을수록 사람).
#[no_mangle]
pub extern "C" fn guard_tick() -> u32 {
    with_engine(|e| e.tick().score as u32)
}

#[no_mangle]
pub extern "C" fn guard_score() -> u32 {
    with_engine(|e| e.verdict().score as u32)
}

/// `guard_scorer::Tier` 값 (0=Block, 1=Challenge, 2=Observe, 3=Trusted).
#[no_mangle]
pub extern "C" fn guard_tier() -> u32 {
    with_engine(|e| e.verdict().tier as u32)
}

#[no_mangle]
pub extern "C" fn guard_sufficient_evidence() -> u32 {
    with_engine(|e| e.has_sufficient_evidence() as u32)
}

#[no_mangle]
pub extern "C" fn guard_feature_count() -> u32 {
    FEATURE_COUNT as u32
}

/// 마지막 `guard_tick()` 시점의 `i`번째 특징. 범위를 벗어나면 NaN.
#[no_mangle]
pub extern "C" fn guard_feature(i: u32) -> f32 {
    with_engine(|e| e.features().0.get(i as usize).copied().unwrap_or(f32::NAN))
}

#[no_mangle]
pub extern "C" fn guard_dropped_events() -> f64 {
    with_engine(|e| e.dropped_events() as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use guard_collector::flags;
    use guard_features::idx;

    #[test]
    fn injected_input_blocks_after_tick() {
        let mut e = Engine::new();
        e.push(
            EventKind::PointerMove as u8,
            flags::TRUSTED | flags::NATIVE_INJECTED,
            0,
            1.0,
            1.0,
            0,
        );
        assert_eq!(e.verdict().tier, Tier::Observe);
        assert_eq!(e.tick().tier, Tier::Block);
    }

    #[test]
    fn ignores_unknown_kind() {
        let mut e = Engine::new();
        e.push(200, 0, 0, 0.0, 0.0, 0);
        e.tick();
        assert_eq!(e.features().0[idx::EVENT_COUNT], 0.0);
    }

    // 각 테스트는 별도 스레드에서 돌므로 thread_local 엔진이 테스트 간에 공유되지 않는다.
    #[test]
    fn c_abi_roundtrip() {
        for i in 0..60 {
            guard_push(
                EventKind::PointerMove as u32,
                0,
                i as f64 * 8000.0,
                i as f32,
                0.0,
                0,
            );
        }
        assert_eq!(guard_score(), 50, "score must not change before tick");
        assert_eq!(guard_tick(), 0);
        assert_eq!(guard_tier(), Tier::Block as u32);
        assert_eq!(guard_feature(idx::EVENT_COUNT as u32), 60.0);
        assert_eq!(guard_feature(idx::UNTRUSTED_RATIO as u32), 1.0);
        assert!(guard_feature(guard_feature_count()).is_nan());
        assert_eq!(guard_sufficient_evidence(), 0);
    }
}
