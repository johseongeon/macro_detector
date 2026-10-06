//! 특징 벡터로부터 신뢰도 점수(0~100, 높을수록 사람)와 조치 단계를 산출한다.
//!
//! 오판 최소화 원칙:
//! - 학습 모델 단독으로는 [`Tier::Block`]을 내리지 않는다. 차단은 결정적 규칙(Stage 1)에서만 나온다.
//! - 증거가 부족하면 점수가 낮아도 [`Tier::Observe`] 이상으로 유지한다.

pub mod calibration;
pub mod gbdt;
pub mod rules;

use guard_features::FeatureVector;
use serde::{Deserialize, Serialize};

pub use calibration::Calibrator;
pub use gbdt::FlatForest;
pub use rules::{HardRule, RuleAction};

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    Block = 0,
    Challenge = 1,
    Observe = 2,
    Trusted = 3,
}

impl Tier {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Block,
            1 => Self::Challenge,
            2 => Self::Observe,
            3 => Self::Trusted,
            _ => return None,
        })
    }
}

/// 점수 → 단계 경계값. 실제 값은 검증 데이터에서 목표 FPR을 만족하도록 정한다.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Thresholds {
    pub trusted: u8,
    pub observe: u8,
    pub challenge: u8,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            trusted: 80,
            observe: 50,
            challenge: 20,
        }
    }
}

impl Thresholds {
    pub fn tier_for(&self, score: u8) -> Tier {
        if score >= self.trusted {
            Tier::Trusted
        } else if score >= self.observe {
            Tier::Observe
        } else if score >= self.challenge {
            Tier::Challenge
        } else {
            Tier::Block
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Verdict {
    /// 0~100, 높을수록 사람일 가능성이 높다.
    pub score: u8,
    pub tier: Tier,
    pub hard_rule: Option<HardRule>,
}

pub struct Scorer {
    forest: FlatForest,
    calibrator: Calibrator,
    thresholds: Thresholds,
}

impl Scorer {
    pub fn new(forest: FlatForest, calibrator: Calibrator, thresholds: Thresholds) -> Self {
        Self {
            forest,
            calibrator,
            thresholds,
        }
    }

    pub fn score(&self, fv: &FeatureVector, sufficient_evidence: bool) -> Verdict {
        let p_bot = self
            .calibrator
            .apply(sigmoid(self.forest.predict_margin(fv.as_slice())));
        let score = ((1.0 - p_bot) * 100.0).round().clamp(0.0, 100.0) as u8;

        if let Some(rule) = rules::check(fv) {
            return match rule.action() {
                RuleAction::Block => Verdict {
                    score: 0,
                    tier: Tier::Block,
                    hard_rule: Some(rule),
                },
                // 점수는 '확인' 구간 안으로 낮추되, 모델이 더 낮게 봤어도 차단하지는 않는다.
                RuleAction::Challenge => Verdict {
                    score: score.min(self.thresholds.observe.saturating_sub(1)),
                    tier: Tier::Challenge,
                    hard_rule: Some(rule),
                },
            };
        }

        let floor = if sufficient_evidence {
            Tier::Challenge
        } else {
            Tier::Observe
        };
        let tier = self.thresholds.tier_for(score).max(floor);

        Verdict {
            score,
            tier,
            hard_rule: None,
        }
    }
}

impl Default for Scorer {
    /// 학습된 모델이 없을 때 쓰는 중립 모델(p_bot = 0.5).
    fn default() -> Self {
        Self::new(
            FlatForest::constant(0.0),
            Calibrator::identity(),
            Thresholds::default(),
        )
    }
}

#[inline]
fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

#[cfg(test)]
mod tests {
    use super::*;
    use guard_features::{idx, FEATURE_COUNT};

    fn fv_with(i: usize, v: f32) -> FeatureVector {
        let mut a = [0.0; FEATURE_COUNT];
        a[i] = v;
        FeatureVector(a)
    }

    #[test]
    fn untrusted_events_block() {
        let mut fv = fv_with(idx::EVENT_COUNT, 100.0);
        fv.0[idx::UNTRUSTED_RATIO] = 0.9;
        let v = Scorer::default().score(&fv, true);
        assert_eq!(v.tier, Tier::Block);
        assert_eq!(v.score, 0);
        assert_eq!(v.hard_rule, Some(HardRule::UntrustedEvents));
    }

    #[test]
    fn injected_input_only_challenges() {
        let v = Scorer::default().score(&fv_with(idx::INJECTED_COUNT, 3.0), true);
        assert_eq!(v.tier, Tier::Challenge);
        assert_eq!(v.score, 49);
        assert_eq!(v.hard_rule, Some(HardRule::InjectedInput));

        // 모델이 매크로로 강하게 의심해도 주입 입력 단독으로는 차단하지 않는다.
        let s = Scorer::new(
            FlatForest::constant(20.0),
            Calibrator::identity(),
            Thresholds::default(),
        );
        let v = s.score(&fv_with(idx::INJECTED_COUNT, 3.0), true);
        assert_eq!(v.tier, Tier::Challenge);
        assert_eq!(v.score, 0);
    }

    #[test]
    fn native_mismatch_only_challenges() {
        let s = Scorer::default();
        assert_eq!(
            s.score(&fv_with(idx::NATIVE_MISMATCH_COUNT, 2.0), true)
                .hard_rule,
            None
        );
        let v = s.score(&fv_with(idx::NATIVE_MISMATCH_COUNT, 3.0), true);
        assert_eq!(v.tier, Tier::Challenge);
        assert_eq!(v.hard_rule, Some(HardRule::NativeMismatch));
    }

    #[test]
    fn untrusted_rule_wins_over_injected() {
        let mut fv = fv_with(idx::EVENT_COUNT, 100.0);
        fv.0[idx::UNTRUSTED_RATIO] = 0.9;
        fv.0[idx::INJECTED_COUNT] = 5.0;
        assert_eq!(Scorer::default().score(&fv, true).tier, Tier::Block);
    }

    #[test]
    fn model_alone_never_blocks() {
        // margin이 매우 커서 p_bot ≈ 1 → score 0
        let s = Scorer::new(
            FlatForest::constant(20.0),
            Calibrator::identity(),
            Thresholds::default(),
        );
        let v = s.score(&fv_with(idx::MOVE_COUNT, 1000.0), true);
        assert_eq!(v.score, 0);
        assert_eq!(v.tier, Tier::Challenge);
    }

    #[test]
    fn insufficient_evidence_stays_observe() {
        let s = Scorer::new(
            FlatForest::constant(20.0),
            Calibrator::identity(),
            Thresholds::default(),
        );
        let v = s.score(&fv_with(idx::MOVE_COUNT, 10.0), false);
        assert_eq!(v.tier, Tier::Observe);
    }

    #[test]
    fn neutral_model_scores_fifty() {
        let v = Scorer::default().score(&fv_with(idx::MOVE_COUNT, 500.0), true);
        assert_eq!(v.score, 50);
        assert_eq!(v.tier, Tier::Observe);
    }
}
