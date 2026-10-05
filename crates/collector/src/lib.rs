//! 입력 이벤트 레코드 정의와 고정 크기 링버퍼.
//!
//! 수집 경로(hot path)에서는 메모리 할당이 일어나지 않도록, 모든 이벤트를
//! 24바이트 고정 크기 [`EventRecord`]로 표현하고 미리 할당된 [`RingBuffer`]에 기록한다.

mod ring;

pub use ring::RingBuffer;

/// 이벤트 종류. JS 쪽 수집기와 값을 공유하므로 번호를 바꾸지 않는다.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    PointerMove = 1,
    PointerDown = 2,
    PointerUp = 3,
    Wheel = 4,
    KeyDown = 5,
    KeyUp = 6,
    Focus = 7,
    Blur = 8,
    VisibilityChange = 9,
}

impl EventKind {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            1 => Self::PointerMove,
            2 => Self::PointerDown,
            3 => Self::PointerUp,
            4 => Self::Wheel,
            5 => Self::KeyDown,
            6 => Self::KeyUp,
            7 => Self::Focus,
            8 => Self::Blur,
            9 => Self::VisibilityChange,
            _ => return None,
        })
    }
}

/// [`EventRecord::flags`] 비트.
pub mod flags {
    /// DOM 이벤트의 `isTrusted == true`.
    pub const TRUSTED: u8 = 1 << 0;
    /// 네이티브 계층에서 `LLMHF_INJECTED` / `LLKHF_INJECTED`가 관측됨.
    pub const NATIVE_INJECTED: u8 = 1 << 1;
    /// `getCoalescedEvents()`로 복원된 중간 샘플.
    pub const COALESCED: u8 = 1 << 2;
}

/// 키 입력의 범주. 개인정보 보호를 위해 실제 키 값은 수집하지 않는다.
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyCategory {
    Other = 0,
    Letter = 1,
    Digit = 2,
    Modifier = 3,
    Navigation = 4,
    Function = 5,
    Whitespace = 6,
}

/// 고정 크기(24바이트) 이벤트 레코드.
///
/// `extra`의 의미는 이벤트 종류에 따라 다르다.
/// - Pointer: 버튼 번호
/// - Key: [`KeyCategory`]
/// - Wheel: delta 부호/크기 버킷
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EventRecord {
    /// 단조 증가 타임스탬프(µs).
    pub t_us: u64,
    pub x: f32,
    pub y: f32,
    pub kind: u8,
    pub flags: u8,
    pub extra: u16,
    pub _reserved: u32,
}

const _: () = assert!(core::mem::size_of::<EventRecord>() == 24);

impl EventRecord {
    pub fn new(kind: EventKind, t_us: u64, x: f32, y: f32, flags: u8, extra: u16) -> Self {
        Self {
            t_us,
            x,
            y,
            kind: kind as u8,
            flags,
            extra,
            _reserved: 0,
        }
    }

    pub fn kind(&self) -> Option<EventKind> {
        EventKind::from_u8(self.kind)
    }

    pub fn is_trusted(&self) -> bool {
        self.flags & flags::TRUSTED != 0
    }

    pub fn is_injected(&self) -> bool {
        self.flags & flags::NATIVE_INJECTED != 0
    }
}
