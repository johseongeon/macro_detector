// 릴리스 빌드에서 콘솔 창을 띄우지 않는다.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod input_guard;

use std::time::Duration;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use tauri::Manager;

/// 모든 페이지 로드 직후, 페이지 스크립트보다 먼저 실행되는 수집기 부트스트랩.
const COLLECTOR_BOOTSTRAP: &str = include_str!("../../bootstrap/collector-bootstrap.js");

/// `build.rs`가 빌드한 WASM 엔진 (crates/wasm).
const ENGINE_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/guard_engine.wasm"));

/// 네이티브 감시 결과를 페이지 수집기에 전달하는 주기.
const NATIVE_REPORT_INTERVAL: Duration = Duration::from_millis(200);

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

            // 네이티브 입력 감시 결과를 주기적으로 페이지 수집기(collector-bootstrap.js)에 전달한다.
            // 불일치 탐지를 위해 값이 0이어도 매 구간 보고한다. 훅이 동작하지 않으면 보고하지 않는다.
            // 수집기가 없는 페이지(허용 목록 밖)에서는 호출이 무시된다.
            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(NATIVE_REPORT_INTERVAL);
                if !input_guard::is_active() {
                    continue;
                }
                // 수집기는 100만을 넘는 보고를 무시하므로 상한을 맞춘다.
                let (injected, moves) = input_guard::take_report();
                let (injected, moves) = (injected.min(1_000_000), moves.min(1_000_000));
                if let Some(window) = handle.get_webview_window("main") {
                    let script = format!("window.__GUARD__?.reportNative?.({injected}, {moves});");
                    let _ = window.eval(&script);
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run guard browser");
}
