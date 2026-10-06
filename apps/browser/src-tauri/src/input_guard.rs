//! Native Input Guard.
//!
//! Windows 저수준 입력 훅(`WH_MOUSE_LL`, `WH_KEYBOARD_LL`)으로 OS 수준에서 주입된 입력
//! (`SendInput`, `mouse_event`, `keybd_event` 등)을 감지한다. DOM에서는 이런 입력도
//! `isTrusted == true`로 보이므로, WASM(DOM 이벤트)만으로는 볼 수 없는 신호다.
//!
//! - 이 앱 창이 포그라운드일 때의 입력만 센다(다른 앱에서 쓰는 도구로 인한 오판 방지).
//! - 훅 프로시저는 시스템 전체 입력 경로에 있으므로 플래그 검사와 원자적 증가만 한다.
//!   (오래 걸리면 Windows가 훅을 조용히 제거하고, 모든 앱의 입력이 느려진다.)
//! - 집계값은 `take_injected()`로 꺼내 페이지 수집기에 전달한다(main.rs).
//!
//! TODO(Phase 1):
//! - `SetCursorPos`처럼 훅을 거치지 않는 커서 이동 감지: 네이티브 입력 없이 DOM 이벤트만
//!   들어오는 불일치(Native–DOM mismatch)로 탐지
//! - Raw Input(`WM_INPUT`)으로 장치 핸들·폴링 주기 수집

use std::sync::atomic::{AtomicU64, Ordering};

static INJECTED: AtomicU64 = AtomicU64::new(0);

/// 마지막 호출 이후 감지된 주입 입력 수를 꺼내고 0으로 초기화한다.
pub fn take_injected() -> u64 {
    INJECTED.swap(0, Ordering::Relaxed)
}

/// 입력 훅 스레드를 시작한다. Windows가 아니면 아무것도 하지 않는다.
pub fn start() {
    #[cfg(windows)]
    imp::start();
}

#[cfg(windows)]
mod imp {
    use std::ptr::null_mut;
    use std::sync::atomic::Ordering;

    use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::System::Threading::GetCurrentProcessId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId,
        SetWindowsHookExW, HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_INJECTED, LLMHF_INJECTED, MSG,
        MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL,
    };

    use super::INJECTED;

    pub fn start() {
        let spawned = std::thread::Builder::new()
            .name("input-guard".into())
            .spawn(|| unsafe {
                let module = GetModuleHandleW(std::ptr::null());
                let mouse = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), module, 0);
                let keyboard = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), module, 0);
                if mouse.is_null() || keyboard.is_null() {
                    // 훅 설치 실패는 탐지 신호가 없을 뿐, 브라우저 동작에는 영향이 없다.
                    eprintln!("[input-guard] failed to install low-level input hooks");
                    return;
                }
                // 저수준 훅은 설치한 스레드가 메시지를 처리해야 호출된다.
                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {}
            });
        if let Err(err) = spawned {
            eprintln!("[input-guard] failed to start: {err}");
        }
    }

    fn app_in_foreground() -> bool {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() {
                return false;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut pid);
            pid == GetCurrentProcessId()
        }
    }

    unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code == HC_ACTION as i32 {
            let info = &*(lparam as *const MSLLHOOKSTRUCT);
            if info.flags & LLMHF_INJECTED != 0 && app_in_foreground() {
                INJECTED.fetch_add(1, Ordering::Relaxed);
            }
        }
        CallNextHookEx(null_mut(), code, wparam, lparam)
    }

    unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code == HC_ACTION as i32 {
            let info = &*(lparam as *const KBDLLHOOKSTRUCT);
            if info.flags & LLKHF_INJECTED != 0 && app_in_foreground() {
                INJECTED.fetch_add(1, Ordering::Relaxed);
            }
        }
        CallNextHookEx(null_mut(), code, wparam, lparam)
    }
}
