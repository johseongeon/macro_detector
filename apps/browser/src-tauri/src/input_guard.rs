//! Native Input Guard.
//!
//! OS 수준 입력을 감시해 `SendInput` 등으로 주입된 합성 입력을 찾아낸다.
//! WASM(DOM 이벤트)만으로는 볼 수 없는 신호이며, 오판 가능성이 매우 낮다.
//!
//! TODO(Phase 1):
//! - Raw Input(`RegisterRawInputDevices`, `WM_INPUT`)으로 장치 핸들·폴링 주기 수집
//! - Low-level hook(`WH_MOUSE_LL`, `WH_KEYBOARD_LL`)의 `LLMHF_INJECTED` / `LLKHF_INJECTED` 플래그 검사
//! - 관측 결과를 QPC 타임스탬프와 함께 WebView 수집기에 전달
//!   (`guard_collector::flags::NATIVE_INJECTED`)

pub fn start() {
    #[cfg(windows)]
    {
        // Windows 전용 구현이 들어갈 자리.
    }
}
