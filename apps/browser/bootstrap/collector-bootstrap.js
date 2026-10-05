// 수집기 부트스트랩. Tauri initialization_script로 모든 페이지에 주입된다.
//
// 브라우저 셸(main.rs)이 이 파일을 IIFE로 감싸고, 그 안에 WASM 엔진 바이트를
// `GUARD_ENGINE_WASM_B64` 상수로 넣어 준다. 그래서 페이지 전역에는 `__GUARD__`만 노출된다.
//
// 이벤트 종류·플래그·키 범주·특징 이름은 crates/collector, crates/features와 반드시 일치해야 한다.
(() => {
  "use strict";

  // TODO(Phase 1): 서버에서 받은 서명된 허용 목록으로 교체
  const ALLOWED_HOSTS = ["localhost", "127.0.0.1"];
  // 점수·특징을 페이지에 노출하는 개발용 호스트. 운영 호스트에서는 절대 노출하지 않는다
  // (매크로가 점수를 읽고 행동을 조정할 수 있으므로).
  const DEBUG_HOSTS = ["localhost", "127.0.0.1"];
  if (!ALLOWED_HOSTS.includes(location.hostname) || window.__GUARD__) return;

  const KIND = { move: 1, down: 2, up: 3, wheel: 4, keydown: 5, keyup: 6, focus: 7, blur: 8, visibility: 9 };
  const FLAG_TRUSTED = 1 << 0;
  const FLAG_COALESCED = 1 << 2;
  const KEY = { other: 0, letter: 1, digit: 2, modifier: 3, navigation: 4, function: 5, whitespace: 6 };
  const TIER_NAMES = ["block", "challenge", "observe", "trusted"];
  const FEATURE_NAMES = [
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

  // 점수 갱신 주기. 예매 클릭 시에는 새로 계산하지 않고 마지막 값을 쓴다.
  // TODO(Phase 1): 계획서대로 SharedArrayBuffer 링버퍼 + Worker로 옮기기
  const TICK_MS = 200;
  // 엔진 로드 전 대기열 상한(이벤트 수). 넘치면 이후 이벤트는 버린다.
  const MAX_PENDING = 8192;
  const STRIDE = 6;

  let engine = null;
  const pending = [];

  // 실제 키 값은 기록하지 않고 범주만 남긴다.
  function keyCategory(e) {
    const c = e.code || "";
    if (c.startsWith("Key")) return KEY.letter;
    if (c.startsWith("Digit") || c.startsWith("Numpad")) return KEY.digit;
    if (/^(Shift|Control|Alt|Meta)/.test(c)) return KEY.modifier;
    if (/^(Arrow|Home|End|Page|Tab)/.test(c)) return KEY.navigation;
    if (/^F\d+$/.test(c)) return KEY.function;
    if (c === "Space" || c === "Enter") return KEY.whitespace;
    return KEY.other;
  }

  function record(kind, e, x, y, extra, extraFlags) {
    const flags = (e.isTrusted ? FLAG_TRUSTED : 0) | (extraFlags || 0);
    const tUs = e.timeStamp * 1000;
    if (engine) {
      engine.guard_push(kind, flags, tUs, x, y, extra);
    } else if (pending.length < MAX_PENDING * STRIDE) {
      pending.push(kind, flags, tUs, x, y, extra);
    }
  }

  async function loadEngine() {
    if (typeof GUARD_ENGINE_WASM_B64 !== "string") {
      throw new Error("engine bytes missing (bootstrap must be injected by the Guard Browser shell)");
    }
    const bytes = Uint8Array.from(atob(GUARD_ENGINE_WASM_B64), (c) => c.charCodeAt(0));
    const { instance } = await WebAssembly.instantiate(bytes, {});
    const ex = instance.exports;
    for (let i = 0; i < pending.length; i += STRIDE) {
      ex.guard_push(pending[i], pending[i + 1], pending[i + 2], pending[i + 3], pending[i + 4], pending[i + 5]);
    }
    pending.length = 0;
    engine = ex;
    engine.guard_tick();
    setInterval(() => engine.guard_tick(), TICK_MS);
  }

  const opts = { passive: true, capture: true };

  addEventListener("pointermove", (e) => {
    const samples = e.getCoalescedEvents ? e.getCoalescedEvents() : [];
    if (samples.length > 1) {
      for (const s of samples) record(KIND.move, s, s.clientX, s.clientY, 0, FLAG_COALESCED);
    } else {
      record(KIND.move, e, e.clientX, e.clientY, 0);
    }
  }, opts);
  addEventListener("pointerdown", (e) => record(KIND.down, e, e.clientX, e.clientY, e.button), opts);
  addEventListener("pointerup", (e) => record(KIND.up, e, e.clientX, e.clientY, e.button), opts);
  addEventListener("wheel", (e) => record(KIND.wheel, e, e.clientX, e.clientY, Math.sign(e.deltaY) + 1), opts);
  addEventListener("keydown", (e) => record(KIND.keydown, e, 0, 0, keyCategory(e)), opts);
  addEventListener("keyup", (e) => record(KIND.keyup, e, 0, 0, keyCategory(e)), opts);
  addEventListener("focus", (e) => record(KIND.focus, e, 0, 0, 0), opts);
  addEventListener("blur", (e) => record(KIND.blur, e, 0, 0, 0), opts);
  document.addEventListener("visibilitychange", (e) => record(KIND.visibility, e, 0, 0, document.hidden ? 1 : 0), opts);

  // 사이트 SDK(sdk/web)가 사용하는 인터페이스.
  const api = {
    version: 1,
    ready: () => engine !== null,
    // TODO(Phase 3): WASM 엔진 점수 + 네이티브 서명으로 실제 토큰 발급
    getToken: async (_nonce) => {
      throw new Error("guard token issuance is not implemented yet");
    },
  };

  if (DEBUG_HOSTS.includes(location.hostname)) {
    api.debug = () => {
      if (!engine) return null;
      const tier = engine.guard_tier();
      return {
        score: engine.guard_score(),
        tier: TIER_NAMES[tier] ?? "unknown",
        sufficientEvidence: engine.guard_sufficient_evidence() === 1,
        droppedEvents: engine.guard_dropped_events(),
        features: Object.fromEntries(FEATURE_NAMES.map((name, i) => [name, engine.guard_feature(i)])),
      };
    };
  }

  Object.defineProperty(window, "__GUARD__", {
    value: Object.freeze(api),
    configurable: false,
    writable: false,
  });

  loadEngine().catch((err) => console.error("[guard] engine load failed:", err));
})();
