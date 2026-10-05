# Guard Browser — 행동 기반 매크로 탐지 보안 브라우저

콘서트·대중교통 예매 사이트 전용 데스크톱 브라우저(.exe)입니다.
WebAssembly로 마우스·키보드 행동을 수집하고 분석해 사용자별 **신뢰도 점수**를 산출합니다.

최우선 목표는 **① 오판 최소화**, **② 최저 지연**입니다. 자세한 내용은 [계획서](plan.html)를 참고하세요.

## 구조

| 경로 | 설명 | 언어 |
|---|---|---|
| `apps/browser/` | Tauri 셸(.exe), 수집기 부트스트랩, 네이티브 입력 감시 | Rust, JS |
| `crates/collector/` | 이벤트 레코드(24바이트)와 고정 크기 링버퍼 | Rust |
| `crates/features/` | 증분 특징 추출기 (이벤트당 O(1)) | Rust |
| `crates/scorer/` | Stage 1 규칙 + Stage 2 GBDT 추론 + 확률 보정 | Rust |
| `crates/token/` | 신뢰도 토큰 서명·검증 (Ed25519) | Rust |
| `crates/wasm/` | 페이지에 주입되는 WASM 엔진 (wasm-bindgen) | Rust |
| `server/verify-api/` | Fast path 토큰 검증 API (axum) | Rust |
| `server/deep-analyzer/` | Stage 3 정밀 분석 서버 (FastAPI) | Python |
| `sdk/web/` | 예매 사이트 연동 SDK | TypeScript |
| `ml/` | 모델 학습, FlatForest 내보내기, FPR 중심 평가 | Python |
| `bench/` | 지연 시간 벤치마크 (criterion) | Rust |
| `testbed/` | 모의 예매 사이트 | HTML/JS |
| `redteam/` | 난이도별 매크로 재현 도구 (내부 전용) | — |

## 시작하기

### 필요 도구
- Rust stable (`rustup`, `wasm32-unknown-unknown` 타깃 — `rust-toolchain.toml`이 자동 설치)
- Node.js 24 이상
- Python 3.11 이상
- 브라우저 셸 빌드: Windows + WebView2, `cargo install tauri-cli`

### 자주 쓰는 명령

```bash
# Rust: 테스트 (브라우저 셸 제외)
cargo test

# WASM 엔진 빌드
wasm-pack build crates/wasm --target web --release

# 지연 벤치마크
cargo bench -p guard-bench

# 검증 API 실행
GUARD_VERIFYING_KEY=<base64url 공개키> cargo run -p guard-verify-api

# 브라우저 셸 (Windows)
cd apps/browser/src-tauri && cargo tauri dev

# 웹 SDK
cd sdk/web && npm install && npm test

# ML
cd ml && pip install -e ".[dev]" && pytest

# 테스트베드
python3 -m http.server 5173 --directory testbed
```

## 언어 간 계약

다음 값은 여러 언어에 중복 정의되어 있으므로 함께 수정해야 합니다.

- 특징 순서: `crates/features/src/lib.rs` `FEATURE_NAMES` ↔ `ml/guard_ml/feature_schema.py` (테스트로 일치 검사)
- 이벤트 종류·플래그·키 범주: `crates/collector/src/lib.rs` ↔ `apps/browser/bootstrap/collector-bootstrap.js`
- 리프 표식 `LEAF`: `crates/scorer/src/gbdt.rs` ↔ `ml/guard_ml/export/flat_forest.py`
- 조치 단계 값: `guard_scorer::Tier` ↔ `crates/wasm`, `server/verify-api`
