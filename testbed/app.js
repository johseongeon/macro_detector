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
