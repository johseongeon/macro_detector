//! Stage 1: 결정적 규칙. 오판 가능성이 사실상 없는 신호만 둔다.
//! 새 규칙은 집단별 FPR 검증을 통과한 뒤에만 추가한다.

use guard_features::{idx, FeatureVector};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HardRule {
    /// OS 수준에서 주입된(SendInput 등) 입력이 관측됨.
    InjectedInput,
    /// 스크립트가 만든(`isTrusted == false`) 이벤트가 대부분을 차지함.
    UntrustedEvents,
}

/// `UntrustedEvents` 판정에 필요한 최소 이벤트 수.
const MIN_EVENTS_FOR_UNTRUSTED_RULE: f32 = 50.0;
const UNTRUSTED_RATIO_LIMIT: f32 = 0.5;

pub fn check(fv: &FeatureVector) -> Option<HardRule> {
    let v = &fv.0;
    if v[idx::INJECTED_COUNT] > 0.0 {
        return Some(HardRule::InjectedInput);
    }
    if v[idx::EVENT_COUNT] >= MIN_EVENTS_FOR_UNTRUSTED_RULE
        && v[idx::UNTRUSTED_RATIO] > UNTRUSTED_RATIO_LIMIT
    {
        return Some(HardRule::UntrustedEvents);
    }
    None
}
