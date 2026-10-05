// 수집기 부트스트랩. Tauri initialization_script로 모든 페이지에 주입된다.
//
// 메인 스레드에서는 이벤트를 숫자 배열에 기록만 한다. 분석은 WASM 엔진(Worker)이 맡는다.
// 이벤트 종류·플래그·키 범주 값은 crates/collector/src/lib.rs와 반드시 일치해야 한다.
(() => {
  "use strict";

  // TODO(Phase 1): 서버에서 받은 서명된 허용 목록으로 교체
  const ALLOWED_HOSTS = ["localhost", "127.0.0.1"];
  if (!ALLOWED_HOSTS.includes(location.hostname) || window.__GUARD__) return;

  const KIND = { move: 1, down: 2, up: 3, wheel: 4, keydown: 5, keyup: 6, focus: 7, blur: 8, visibility: 9 };
  const FLAG_TRUSTED = 1 << 0;
  const FLAG_COALESCED = 1 << 2;
  const KEY = { other: 0, letter: 1, digit: 2, modifier: 3, navigation: 4, function: 5, whitespace: 6 };

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

  // [kind, flags, t_us, x, y, extra] 6칸씩. TODO(Phase 1): SharedArrayBuffer 링버퍼 + Worker로 교체
  const STRIDE = 6;
  const CAPACITY = 8192;
  const buf = new Float64Array(CAPACITY * STRIDE);
  let head = 0;
  let len = 0;

  function record(kind, e, x, y, extra, extraFlags) {
    const o = head * STRIDE;
    buf[o] = kind;
    buf[o + 1] = (e.isTrusted ? FLAG_TRUSTED : 0) | (extraFlags || 0);
    buf[o + 2] = e.timeStamp * 1000;
    buf[o + 3] = x;
    buf[o + 4] = y;
    buf[o + 5] = extra;
    head = (head + 1) % CAPACITY;
    if (len < CAPACITY) len++;
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
  Object.defineProperty(window, "__GUARD__", {
    value: Object.freeze({
      version: 1,
      bufferedEvents: () => len,
      // TODO(Phase 3): WASM 엔진 점수 + 네이티브 서명으로 실제 토큰 발급
      getToken: async (_nonce) => {
        throw new Error("guard token issuance is not implemented yet");
      },
    }),
    configurable: false,
    writable: false,
  });
})();
