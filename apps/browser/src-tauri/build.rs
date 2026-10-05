use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    build_engine_wasm();
    tauri_build::build()
}

/// 수집기에 내장할 WASM 엔진(`crates/wasm`)을 빌드해 `OUT_DIR/guard_engine.wasm`에 둔다.
///
/// 바깥 빌드와 빌드 디렉터리 잠금이 충돌하지 않도록 별도 target 디렉터리를 쓴다.
/// `wasm32-unknown-unknown` 타깃은 rust-toolchain.toml이 설치한다.
fn build_engine_wasm() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace = manifest_dir.join("..").join("..").join("..");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target_dir = out_dir.join("wasm-target");
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());

    let status = Command::new(cargo)
        .current_dir(&workspace)
        .args([
            "build",
            "-p",
            "guard-wasm",
            "--release",
            "--target",
            "wasm32-unknown-unknown",
        ])
        .arg("--target-dir")
        .arg(&target_dir)
        // 바깥 빌드의 호스트용 플래그가 wasm 빌드에 섞이지 않게 한다.
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("RUSTFLAGS")
        .status()
        .expect("failed to run cargo for guard-wasm");
    assert!(
        status.success(),
        "guard-wasm build failed (is the wasm32-unknown-unknown target installed?)"
    );

    std::fs::copy(
        target_dir.join("wasm32-unknown-unknown/release/guard_wasm.wasm"),
        out_dir.join("guard_engine.wasm"),
    )
    .expect("failed to copy guard_wasm.wasm");

    for path in [
        "Cargo.toml",
        "Cargo.lock",
        "crates/collector",
        "crates/features",
        "crates/scorer",
        "crates/wasm",
    ] {
        println!("cargo:rerun-if-changed={}", workspace.join(path).display());
    }
}
