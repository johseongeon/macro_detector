// 모의 예매 흐름: 카운트다운 → 좌석 선택 → 예매 버튼.
// 보안 브라우저가 주입한 window.__GUARD__가 있으면 예매 시 토큰을 요청한다.

const OPEN_DELAY_SEC = 10;
const ROWS = 6;
const COLS = 12;

const seatsEl = document.getElementById("seats");
const bookBtn = document.getElementById("book");
const resultEl = document.getElementById("result");
const countdownEl = document.getElementById("countdown");
const guardEl = document.getElementById("guard-status");

let selected = null;

for (let r = 0; r < ROWS; r++) {
  for (let c = 0; c < COLS; c++) {
    const seat = document.createElement("button");
    seat.className = "seat";
    seat.setAttribute("aria-label", `${String.fromCharCode(65 + r)}열 ${c + 1}번`);
    seat.setAttribute("aria-pressed", "false");
    seat.disabled = Math.random() < 0.3;
    seat.addEventListener("click", () => {
      selected?.setAttribute("aria-pressed", "false");
      selected = seat;
      seat.setAttribute("aria-pressed", "true");
    });
    seatsEl.append(seat);
  }
}

guardEl.textContent = window.__GUARD__ ? "보안 브라우저 감지됨" : "일반 브라우저 (수집 비활성)";

let remaining = OPEN_DELAY_SEC;
countdownEl.textContent = remaining;
const timer = setInterval(() => {
  remaining -= 1;
  countdownEl.textContent = Math.max(remaining, 0);
  if (remaining <= 0) {
    clearInterval(timer);
    bookBtn.disabled = false;
  }
}, 1000);

bookBtn.addEventListener("click", async () => {
  if (!selected) {
    resultEl.textContent = "좌석을 먼저 선택하세요.";
    return;
  }
  if (!window.__GUARD__) {
    resultEl.textContent = "예매 요청(토큰 없음) — 실제 서비스에서는 확인 단계로 처리됩니다.";
    return;
  }
  try {
    // TODO(Phase 3): 서버에서 발급한 nonce 사용 및 verify-api 호출
    const token = await window.__GUARD__.getToken(crypto.randomUUID());
    resultEl.textContent = `토큰 발급 완료 (${token.length} bytes)`;
  } catch (err) {
    resultEl.textContent = `토큰 발급 실패: ${err.message}`;
  }
});

// ---- 개발용 신뢰도 패널 ----
// __GUARD__.debug()는 개발용 호스트(localhost)에서만 제공된다.

const TIER_LABELS = { trusted: "신뢰", observe: "관찰", challenge: "확인", block: "차단" };
const RULE_LABELS = {
  injected_input: "규칙: OS 수준 주입 입력 감지",
  untrusted_events: "규칙: 스크립트 생성 이벤트 감지",
  native_mismatch: "규칙: OS 입력 없는 마우스 이동 감지",
};
const SHOWN_FEATURES = {
  event_count: "이벤트 수",
  move_count: "마우스 이동",
  speed_mean: "평균 속도 (px/ms)",
  speed_std: "속도 표준편차",
  click_interval_mean_ms: "클릭 간격 평균 (ms)",
  key_dwell_mean_ms: "키 누름 평균 (ms)",
  untrusted_ratio: "비신뢰 이벤트 비율",
  injected_count: "주입 입력 수",
  native_mismatch_count: "OS–DOM 불일치 구간",
};

const debugEl = document.getElementById("debug");
const scoreEl = document.getElementById("score");
const tierEl = document.getElementById("tier");
const ruleEl = document.getElementById("rule");
const meterEl = document.getElementById("meter-fill");
const featuresEl = document.getElementById("features");

const featureCells = {};
for (const [key, label] of Object.entries(SHOWN_FEATURES)) {
  const row = document.createElement("div");
  const dt = document.createElement("dt");
  const dd = document.createElement("dd");
  dt.textContent = label;
  row.append(dt, dd);
  featuresEl.append(row);
  featureCells[key] = dd;
}

function renderDebug() {
  const d = window.__GUARD__?.debug?.();
  if (!d) return;
  debugEl.hidden = false;
  scoreEl.textContent = d.score;
  tierEl.dataset.tier = d.tier;
  tierEl.textContent = TIER_LABELS[d.tier] ?? d.tier;
  ruleEl.textContent = d.rule ? RULE_LABELS[d.rule] ?? d.rule : "";
  meterEl.style.width = `${d.score}%`;
  for (const [key, cell] of Object.entries(featureCells)) {
    const v = d.features[key];
    cell.textContent = Number.isInteger(v) ? v : v.toFixed(3);
  }
}
setInterval(renderDebug, 250);

// 스크립트로 직선 이동 + 클릭을 흉내 낸다. dispatchEvent로 만든 이벤트는 isTrusted=false다.
document.getElementById("simulate").addEventListener("click", () => {
  const target = document.querySelector(".seat:not(:disabled)");
  const rect = target.getBoundingClientRect();
  const endX = rect.left + rect.width / 2;
  const endY = rect.top + rect.height / 2;
  const steps = 300;
  for (let i = 0; i <= steps; i++) {
    const init = { bubbles: true, clientX: (endX * i) / steps, clientY: (endY * i) / steps };
    document.body.dispatchEvent(new PointerEvent("pointermove", init));
  }
  const at = { bubbles: true, clientX: endX, clientY: endY };
  target.dispatchEvent(new PointerEvent("pointerdown", at));
  target.dispatchEvent(new PointerEvent("pointerup", at));
  target.click();
});
