// 수집기 부트스트랩 + 실제 WASM 엔진 통합 테스트.
// 실행 전 WASM 빌드 필요: cargo build -p guard-wasm --target wasm32-unknown-unknown --release
// 경로가 다르면 GUARD_ENGINE_WASM 환경 변수로 지정한다.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const here = dirname(fileURLToPath(import.meta.url));
const wasmPath =
  process.env.GUARD_ENGINE_WASM ?? join(here, "../../../target/wasm32-unknown-unknown/release/guard_wasm.wasm");
const wasmB64 = readFileSync(wasmPath).toString("base64");
const bootstrap = readFileSync(join(here, "collector-bootstrap.js"), "utf8");

/** main.rs의 collector_script()와 같은 방식으로 감싸 가짜 페이지에서 실행한다. */
function loadPage(hostname = "localhost") {
  const listeners = {};
  const intervals = [];
  const ctx = {
    location: { hostname },
    addEventListener: (type, fn) => (listeners[type] = fn),
    document: { hidden: false, addEventListener: () => {} },
    atob,
    WebAssembly,
    console,
    setInterval: (fn) => intervals.push(fn),
  };
  ctx.window = ctx;
  vm.createContext(ctx);
  vm.runInContext(`(() => {\nconst GUARD_ENGINE_WASM_B64 = "${wasmB64}";\n${bootstrap}\n})();`, ctx);

  let t = 0;
  return {
    guard: () => ctx.__GUARD__,
    move: (n, isTrusted = true, pointerType = "mouse") => {
      for (let i = 0; i < n; i++) {
        t += 8;
        listeners.pointermove({ isTrusted, pointerType, timeStamp: t, clientX: i * 3, clientY: 100 });
      }
    },
    tick: () => intervals.forEach((fn) => fn()),
  };
}

async function ready(page) {
  for (let i = 0; i < 200 && !page.guard().ready(); i++) await new Promise((r) => setTimeout(r, 5));
  assert.ok(page.guard().ready(), "engine did not load");
}

test("engine loads with no imports and replays events queued before load", async () => {
  const page = loadPage();
  page.move(20); // 엔진 로드(비동기) 전에 들어온 이벤트
  assert.equal(page.guard().debug(), null);
  await ready(page);
  page.move(10);
  page.tick();

  const d = page.guard().debug();
  assert.equal(d.features.event_count, 30);
  assert.equal(d.features.move_count, 30);
  assert.equal(d.features.untrusted_ratio, 0);
  assert.equal(d.score, 50);
  assert.equal(d.tier, "observe");
  assert.equal(d.sufficientEvidence, false);
});

test("untrusted (script-generated) events trigger the hard rule", async () => {
  const page = loadPage();
  await ready(page);
  page.move(60, false);
  page.tick();

  const d = page.guard().debug();
  assert.equal(d.tier, "block");
  assert.equal(d.rule, "untrusted_events");
  assert.equal(d.score, 0);
});

test("native injected reports challenge but never block", async () => {
  const page = loadPage();
  await ready(page);
  page.move(10);
  page.guard().reportNative(3, 10);
  page.guard().reportNative(-1, 10); // 잘못된 값은 무시
  page.guard().reportNative(1.5, 10);
  page.guard().reportNative(2, 5);
  page.tick();

  const d = page.guard().debug();
  assert.equal(d.features.injected_count, 5);
  assert.equal(d.features.native_mismatch_count, 0);
  assert.equal(d.tier, "challenge");
  assert.equal(d.rule, "injected_input");
  assert.equal(d.score, 49);
});

test("trusted mouse moves the OS never saw are flagged as native mismatch", async () => {
  const page = loadPage();
  await ready(page);
  page.guard().reportNative(0, 0); // 기준 구간
  for (let i = 0; i < 3; i++) {
    page.move(10);
    page.guard().reportNative(0, 0);
  }
  page.tick();

  const d = page.guard().debug();
  assert.equal(d.features.native_mismatch_count, 3);
  assert.equal(d.tier, "challenge");
  assert.equal(d.rule, "native_mismatch");
});

test("touch moves and moves the OS saw are not mismatches", async () => {
  const page = loadPage();
  await ready(page);
  page.guard().reportNative(0, 0);
  for (let i = 0; i < 3; i++) {
    page.move(10, true, "touch");
    page.guard().reportNative(0, 0);
  }
  for (let i = 0; i < 3; i++) {
    page.move(10);
    page.guard().reportNative(0, 12);
  }
  page.tick();

  const d = page.guard().debug();
  assert.equal(d.features.native_mismatch_count, 0);
  assert.equal(d.rule, null);
});

test("does nothing on hosts outside the allowlist", () => {
  const page = loadPage("example.com");
  assert.equal(page.guard(), undefined);
});
