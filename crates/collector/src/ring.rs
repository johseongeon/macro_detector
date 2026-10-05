use crate::EventRecord;

/// 단일 생산자/단일 소비자용 고정 크기 링버퍼.
///
/// 가득 차면 가장 오래된 이벤트를 덮어쓰고 `dropped`를 증가시킨다.
/// 수집이 분석보다 우선이므로 생산자는 절대 블로킹되지 않는다.
///
/// TODO(Phase 1): SharedArrayBuffer + Atomics 기반으로 메인 스레드(쓰기)와
/// Worker(읽기)가 버퍼를 공유하는 lock-free 버전으로 교체.
pub struct RingBuffer {
    buf: Box<[EventRecord]>,
    mask: usize,
    head: usize,
    len: usize,
    dropped: u64,
}

impl RingBuffer {
    /// `capacity`는 2의 거듭제곱이어야 한다.
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(
            capacity.is_power_of_two(),
            "capacity must be a power of two"
        );
        Self {
            buf: vec![EventRecord::default(); capacity].into_boxed_slice(),
            mask: capacity - 1,
            head: 0,
            len: 0,
            dropped: 0,
        }
    }

    #[inline]
    pub fn push(&mut self, rec: EventRecord) {
        self.buf[self.head] = rec;
        self.head = (self.head + 1) & self.mask;
        if self.len == self.buf.len() {
            self.dropped += 1;
        } else {
            self.len += 1;
        }
    }

    /// 쌓인 이벤트를 오래된 순서대로 `f`에 전달하고 버퍼를 비운다.
    pub fn drain(&mut self, mut f: impl FnMut(&EventRecord)) {
        let start = (self.head + self.buf.len() - self.len) & self.mask;
        for i in 0..self.len {
            f(&self.buf[(start + i) & self.mask]);
        }
        self.len = 0;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn capacity(&self) -> usize {
        self.buf.len()
    }

    /// 버퍼가 가득 차 덮어쓴 이벤트 수.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EventKind;

    fn rec(t: u64) -> EventRecord {
        EventRecord::new(EventKind::PointerMove, t, 0.0, 0.0, 0, 0)
    }

    fn drained(rb: &mut RingBuffer) -> Vec<u64> {
        let mut out = Vec::new();
        rb.drain(|e| out.push(e.t_us));
        out
    }

    #[test]
    fn drains_in_insertion_order() {
        let mut rb = RingBuffer::with_capacity(4);
        rb.push(rec(1));
        rb.push(rec(2));
        rb.push(rec(3));
        assert_eq!(drained(&mut rb), vec![1, 2, 3]);
        assert!(rb.is_empty());
    }

    #[test]
    fn overwrites_oldest_when_full() {
        let mut rb = RingBuffer::with_capacity(4);
        for t in 1..=6 {
            rb.push(rec(t));
        }
        assert_eq!(rb.dropped(), 2);
        assert_eq!(drained(&mut rb), vec![3, 4, 5, 6]);
    }

    #[test]
    fn wraps_around_after_drain() {
        let mut rb = RingBuffer::with_capacity(4);
        for t in 1..=3 {
            rb.push(rec(t));
        }
        drained(&mut rb);
        for t in 4..=7 {
            rb.push(rec(t));
        }
        assert_eq!(drained(&mut rb), vec![4, 5, 6, 7]);
        assert_eq!(rb.dropped(), 0);
    }
}
