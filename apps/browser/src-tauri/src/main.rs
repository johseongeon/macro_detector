// 릴리스 빌드에서 콘솔 창을 띄우지 않는다.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod input_guard;

use base64::{engine::general_purpose::STANDARD, Engine as _};

/// 모든 페이지 로드 직후, 페이지 스크립트보다 먼저 실행되는 수집기 부트스트랩.
const COLLECTOR_BOOTSTRAP: &str = include_str!("../../bootstrap/collector-bootstrap.js");

/// `build.rs`가 빌드한 WASM 엔진 (crates/wasm).
const ENGINE_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/guard_engine.wasm"));

/// WASM 바이트를 base64로 넣고 전체를 IIFE로 감싸, 페이지 전역 스코프에 아무것도 남기지 않는다.
fn collector_script() -> String {
    format!(
        "(() => {{\nconst GUARD_ENGINE_WASM_B64 = \"{}\";\n{}\n}})();",
        STANDARD.encode(ENGINE_WASM),
        COLLECTOR_BOOTSTRAP
    )
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            input_guard::start();

            // DevTools는 tauri의 `devtools` feature 없이 빌드한 릴리스에서 비활성화된다.
            // TODO(Phase 4): WebView2 원격 디버깅 포트 차단 및 자동화 연결 감지.
            tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("Guard Browser")
            .inner_size(1280.0, 800.0)
            .initialization_script(collector_script())
            .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run guard browser");
}
