//! 이벤트 스트림에서 행동 특징을 증분 추출한다.
//!
//! 모든 특징은 이벤트 한 건당 O(1)로 갱신되며, 원본 시퀀스를 다시 처리하지 않는다.
//! 특징 순서는 모델 학습 코드(`ml/guard_ml/feature_schema.py`)와 반드시 일치해야 한다.

mod stats;

pub use stats::RunningStats;

use guard_collector::{EventKind, EventRecord};

/// 특징 벡터의 인덱스. 순서를 바꾸면 모델을 다시 학습해야 한다.
pub mod idx {
    pub const EVENT_COUNT: usize = 0;
    pub const MOVE_COUNT: usize = 1;
    pub const SPEED_MEAN: usize = 2;
    pub const SPEED_STD: usize = 3;
    pub const CLICK_INTERVAL_MEAN_MS: usize = 4;
    pub const CLICK_INTERVAL_STD_MS: usize = 5;
    pub const KEY_DWELL_MEAN_MS: usize = 6;
    pub const KEY_DWELL_STD_MS: usize = 7;
    pub const UNTRUSTED_RATIO: usize = 8;
    pub const INJECTED_COUNT: usize = 9;
}

pub const FEATURE_COUNT: usize = 10;

pub const FEATURE_NAMES: [&str; FEATURE_COUNT] = [
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
];

/// 낮은 점수를 내리기 위한 최소 마우스 이동 이벤트 수.
/// 증거가 부족한 상태에서 판정하면 오판이 늘어나므로, 이 값 미만이면 '관찰'로만 둔다.
pub const MIN_MOVES_FOR_VERDICT: u64 = 300;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FeatureVector(pub [f32; FEATURE_COUNT]);

impl FeatureVector {
    pub fn as_slice(&self) -> &[f32] {
        &self.0
    }
}

/// 키 범주별 슬롯 수. [`guard_collector::KeyCategory`]의 값 범위를 덮는다.
const KEY_SLOTS: usize = 8;

#[derive(Debug, Default)]
pub struct FeatureExtractor {
    event_count: u64,
    untrusted_count: u64,
    injected_count: u64,

    last_move: Option<(u64, f32, f32)>,
    speed: RunningStats,

    last_down_us: Option<u64>,
    click_interval: RunningStats,

    // TODO(Phase 2): 범주 단위 매칭은 동시 누름(rollover)에서 부정확하다.
    // 개인정보를 해치지 않는 키 식별 방식(세션 단위 임시 ID 등)을 검토.
    key_down_us: [Option<u64>; KEY_SLOTS],
    key_dwell: RunningStats,
}

impl FeatureExtractor {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn update(&mut self, e: &EventRecord) {
        self.event_count += 1;
        if !e.is_trusted() {
            self.untrusted_count += 1;
        }
        if e.is_injected() {
            self.injected_count += 1;
        }

        match e.kind() {
            Some(EventKind::PointerMove) => self.on_move(e),
            Some(EventKind::PointerDown) => self.on_down(e),
            Some(EventKind::KeyDown) => {
                if let Some(slot) = self.key_down_us.get_mut(e.extra as usize) {
                    // 키 반복(autorepeat)은 첫 누름 시각을 유지한다.
                    slot.get_or_insert(e.t_us);
                }
            }
            Some(EventKind::KeyUp) => {
                if let Some(down) = self
                    .key_down_us
                    .get_mut(e.extra as usize)
                    .and_then(Option::take)
                {
                    self.key_dwell.push(us_to_ms(e.t_us.saturating_sub(down)));
                }
            }
            _ => {}
        }
    }

    fn on_move(&mut self, e: &EventRecord) {
        if let Some((t, x, y)) = self.last_move {
            let dt_ms = us_to_ms(e.t_us.saturating_sub(t));
            if dt_ms > 0.0 {
                let dist = ((e.x - x).powi(2) + (e.y - y).powi(2)).sqrt() as f64;
                self.speed.push(dist / dt_ms);
            }
        }
        self.last_move = Some((e.t_us, e.x, e.y));
    }

    fn on_down(&mut self, e: &EventRecord) {
        if let Some(prev) = self.last_down_us {
            self.click_interval
                .push(us_to_ms(e.t_us.saturating_sub(prev)));
        }
        self.last_down_us = Some(e.t_us);
    }

    pub fn move_count(&self) -> u64 {
        // 속도 표본 수 + 첫 이동 1건
        self.speed.count() + self.last_move.is_some() as u64
    }

    pub fn has_sufficient_evidence(&self) -> bool {
        self.move_count() >= MIN_MOVES_FOR_VERDICT
    }

    pub fn snapshot(&self) -> FeatureVector {
        let mut v = [0.0f32; FEATURE_COUNT];
        v[idx::EVENT_COUNT] = self.event_count as f32;
        v[idx::MOVE_COUNT] = self.move_count() as f32;
        v[idx::SPEED_MEAN] = self.speed.mean() as f32;
        v[idx::SPEED_STD] = self.speed.std() as f32;
        v[idx::CLICK_INTERVAL_MEAN_MS] = self.click_interval.mean() as f32;
        v[idx::CLICK_INTERVAL_STD_MS] = self.click_interval.std() as f32;
        v[idx::KEY_DWELL_MEAN_MS] = self.key_dwell.mean() as f32;
        v[idx::KEY_DWELL_STD_MS] = self.key_dwell.std() as f32;
        v[idx::UNTRUSTED_RATIO] = if self.event_count == 0 {
            0.0
        } else {
            self.untrusted_count as f32 / self.event_count as f32
        };
        v[idx::INJECTED_COUNT] = self.injected_count as f32;
        FeatureVector(v)
    }
}

#[inline]
fn us_to_ms(us: u64) -> f64 {
    us as f64 / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use guard_collector::{flags, KeyCategory};

    fn ev(kind: EventKind, t_ms: u64, x: f32, extra: u16, fl: u8) -> EventRecord {
        EventRecord::new(kind, t_ms * 1000, x, 0.0, fl, extra)
    }

    #[test]
    fn constant_speed_has_zero_std() {
        let mut fx = FeatureExtractor::new();
        for i in 0..10 {
            fx.update(&ev(
                EventKind::PointerMove,
                i * 10,
                i as f32 * 20.0,
                0,
                flags::TRUSTED,
            ));
        }
        let v = fx.snapshot();
        assert_eq!(v.0[idx::MOVE_COUNT], 10.0);
        assert!((v.0[idx::SPEED_MEAN] - 2.0).abs() < 1e-6);
        assert!(v.0[idx::SPEED_STD].abs() < 1e-6);
    }

    #[test]
    fn measures_click_interval_and_key_dwell() {
        let mut fx = FeatureExtractor::new();
        let letter = KeyCategory::Letter as u16;
        fx.update(&ev(EventKind::PointerDown, 0, 0.0, 0, flags::TRUSTED));
        fx.update(&ev(EventKind::PointerDown, 250, 0.0, 0, flags::TRUSTED));
        fx.update(&ev(EventKind::KeyDown, 300, 0.0, letter, flags::TRUSTED));
        fx.update(&ev(EventKind::KeyDown, 330, 0.0, letter, flags::TRUSTED)); // autorepeat
        fx.update(&ev(EventKind::KeyUp, 380, 0.0, letter, flags::TRUSTED));
        let v = fx.snapshot();
        assert_eq!(v.0[idx::CLICK_INTERVAL_MEAN_MS], 250.0);
        assert_eq!(v.0[idx::KEY_DWELL_MEAN_MS], 80.0);
    }

    #[test]
    fn counts_untrusted_and_injected() {
        let mut fx = FeatureExtractor::new();
        fx.update(&ev(EventKind::PointerMove, 0, 0.0, 0, flags::TRUSTED));
        fx.update(&ev(EventKind::PointerMove, 1, 0.0, 0, 0));
        fx.update(&ev(
            EventKind::PointerMove,
            2,
            0.0,
            0,
            flags::TRUSTED | flags::NATIVE_INJECTED,
        ));
        let v = fx.snapshot();
        assert!((v.0[idx::UNTRUSTED_RATIO] - 1.0 / 3.0).abs() < 1e-6);
        assert_eq!(v.0[idx::INJECTED_COUNT], 1.0);
        assert!(!fx.has_sufficient_evidence());
    }
}
