//! Stage 1: 결정적 규칙.
//! 새 규칙은 집단별 FPR 검증을 통과한 뒤에만 추가한다.
//!
//! 규칙마다 조치 수준이 다르다. 정상 사용자가 걸릴 수 있는 경로가 하나라도 알려진 규칙은
//! 차단하지 않고 '확인' 단계로만 보낸다.

use guard_features::{idx, FeatureVector};

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HardRule {
    /// OS 수준에서 주입된(SendInput 등) 입력이 관측됨.
    InjectedInput = 1,
    /// 스크립트가 만든(`isTrusted == false`) 이벤트가 대부분을 차지함.
    UntrustedEvents = 2,
    /// DOM에는 신뢰된 마우스 이동이 들어왔지만 OS 입력 훅은 이동을 보지 못함
    /// (`SetCursorPos`, DevTools 프로토콜 등 하드웨어를 거치지 않은 입력).
    NativeMismatch = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleAction {
    Block,
    Challenge,
}

impl HardRule {
    pub fn action(self) -> RuleAction {
        match self {
            // 화상 키보드(osk.exe), 시선 추적기, 음성 입력 등 보조 기기도 입력을 주입한다.
            // 접근성 사용자를 차단하지 않도록 단독으로는 '확인'까지만.
            // TODO(Phase 2): 다른 독립 신호와 함께 관측되면 차단으로 상향.
            HardRule::InjectedInput => RuleAction::Challenge,
            // 원격 데스크톱 등 OS 훅이 이동을 보지 못하는 정상 환경이 있을 수 있고,
            // Windows가 느린 훅을 조용히 제거하면 모든 사용자가 불일치로 보인다.
            // TODO(Phase 5): 원격 데스크톱·보조 기기 집단에서 FPR 검증
            HardRule::NativeMismatch => RuleAction::Challenge,
            // 페이지 스크립트가 만든 이벤트는 실제 사용자 입력일 수 없다.
            HardRule::UntrustedEvents => RuleAction::Block,
        }
    }
}

/// `UntrustedEvents` 판정에 필요한 최소 이벤트 수.
const MIN_EVENTS_FOR_UNTRUSTED_RULE: f32 = 50.0;
const UNTRUSTED_RATIO_LIMIT: f32 = 0.5;
/// 불일치 구간이 이만큼 쌓여야 규칙을 적용한다(보고 주기 200ms 기준 약 0.6초 이상).
const MIN_MISMATCH_INTERVALS: f32 = 3.0;

/// 걸린 규칙 중 가장 강한 조치의 규칙을 반환한다.
pub fn check(fv: &FeatureVector) -> Option<HardRule> {
    let v = &fv.0;
    if v[idx::EVENT_COUNT] >= MIN_EVENTS_FOR_UNTRUSTED_RULE
        && v[idx::UNTRUSTED_RATIO] > UNTRUSTED_RATIO_LIMIT
    {
        return Some(HardRule::UntrustedEvents);
    }
    if v[idx::INJECTED_COUNT] > 0.0 {
        return Some(HardRule::InjectedInput);
    }
    if v[idx::NATIVE_MISMATCH_COUNT] >= MIN_MISMATCH_INTERVALS {
        return Some(HardRule::NativeMismatch);
    }
    None
}
