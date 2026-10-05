#!/usr/bin/env bash
# WSL에서 Windows용 Guard Browser(.exe)와 NSIS 설치 파일을 빌드한다.
#
# WSL 경로(\\wsl$)에서 직접 빌드하면 느리고 불안정하므로, 소스를 Windows 임시 폴더로
# 복사해 Windows 툴체인(MSVC)으로 빌드한 뒤 결과물을 dist/로 가져온다.
# 빌드 캐시(target/)는 Windows 쪽에 남겨 두어 다음 빌드를 빠르게 한다.
#
# 사용법: scripts/build-windows.sh [--no-installer]
#
# 필요 도구(Windows): Rust(rustup), MSVC 빌드 도구, tauri-cli(`cargo install tauri-cli --version "^2" --locked`)
# Windows에서 직접 빌드할 때는 이 스크립트 없이 apps/browser/src-tauri에서 `cargo tauri build`를 실행한다.
set -euo pipefail

installer=1
case "${1:-}" in
  "") ;;
  --no-installer) installer=0 ;;
  *) echo "usage: $0 [--no-installer]" >&2; exit 2 ;;
esac

command -v cmd.exe >/dev/null || { echo "error: WSL에서만 동작합니다 (cmd.exe 없음)" >&2; exit 1; }

repo="$(cd "$(dirname "$0")/.." && pwd)"
win_local="$(cd /mnt/c && cmd.exe /c "echo %LOCALAPPDATA%" 2>/dev/null | tr -d '\r')"
build_win="$win_local\\Temp\\guard-build"
build="$(wslpath -u "$build_win")"

win() {
  # 따옴표·공백이 WSL→cmd 전달 과정에서 깨지지 않도록 임시 .cmd 파일로 실행한다.
  mkdir -p "$build"
  printf '@echo off\r\nset "PATH=%%USERPROFILE%%\\.cargo\\bin;%%PATH%%"\r\nset "CARGO_TARGET_DIR=%s\\target"\r\n%s\r\n' \
    "$build_win" "$*" > "$build/run.cmd"
  (cd /mnt/c && cmd.exe /c "$build_win\\run.cmd")
}

win "cargo --version" >/dev/null 2>&1 || {
  echo "error: Windows에 Rust가 없습니다. https://rustup.rs 에서 설치하세요." >&2; exit 1; }
if [ "$installer" = 1 ] && ! win "cargo tauri --version" >/dev/null 2>&1; then
  echo "error: tauri-cli가 없습니다. Windows에서 'cargo install tauri-cli --version \"^2\" --locked' 실행 후 다시 시도하세요." >&2
  exit 1
fi

echo "==> 소스 복사: $build_win\\src"
rm -rf "$build/src"
mkdir -p "$build/src"
tar -C "$repo" --exclude=./.git --exclude=./target --exclude=./dist \
  --exclude='*/node_modules' --exclude='*/__pycache__' -cf - . | tar -C "$build/src" -xf -

if [ "$installer" = 1 ]; then
  echo "==> cargo tauri build (exe + NSIS 설치 파일)"
  win "cd /d $build_win\\src\\apps\\browser\\src-tauri && cargo tauri build --bundles nsis" | tr -d '\r'
else
  echo "==> cargo build --release (exe만)"
  win "cd /d $build_win\\src && cargo build --release -p guard-browser --features tauri/custom-protocol" | tr -d '\r'
fi

mkdir -p "$repo/dist"
cp "$build/target/release/guard-browser.exe" "$repo/dist/"
if [ "$installer" = 1 ]; then
  cp "$build"/target/release/bundle/nsis/*-setup.exe "$repo/dist/"
fi

echo "==> 완료"
ls -la "$repo/dist"
