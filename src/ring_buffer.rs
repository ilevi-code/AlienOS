pub struct RingBuffer<const N: usize> {
    producer: usize,
    used_len: usize,
    data: [u8; N],
}

impl<const N: usize> RingBuffer<N> {
    #[cfg(test)]
    fn new() -> Self {
        Self {
            producer: 0,
            used_len: 0,
            data: [0; N],
        }
    }

    pub fn push(&mut self, val: u8) {
        if self.used_len == N {
            return;
        }
        self.data[self.producer] = val;
        self.producer = (self.producer + 1) % N;
        self.used_len += 1;
    }

    pub fn free_len(&mut self) -> usize {
        N - self.used_len
    }

    pub fn pop(&mut self) -> Option<u8> {
        if self.used_len == 0 {
            return None;
        }
        let consumer = (self.producer + N - self.used_len) % N;
        self.used_len -= 1;
        Some(self.data[consumer])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn single_push() {
        let mut buf = RingBuffer::<10>::new();
        buf.push(2);
        assert_eq!(buf.producer, 1);
        assert_eq!(buf.used_len, 1);
        assert_eq!(buf.data[0], 2);
    }

    #[test_case]
    fn push_pop() {
        let mut buf = RingBuffer::<10>::new();
        buf.push(2);
        assert_eq!(buf.pop(), Some(2));
        assert_eq!(buf.producer, 1);
        assert_eq!(buf.used_len, 0);
    }

    #[test_case]
    fn push_pop_push() {
        let mut buf = RingBuffer::<10>::new();
        buf.push(2);
        let _ = buf.pop();
        buf.push(3);
        assert_eq!(buf.producer, 2);
        assert_eq!(buf.used_len, 1);
        assert_eq!(buf.data[1], 3);
    }

    #[test_case]
    fn pop_when_empty() {
        let mut buf = RingBuffer::<3>::new();
        assert_eq!(buf.pop(), None);
    }

    #[test_case]
    fn saturation() {
        let mut buf = RingBuffer::<3>::new();
        buf.push(3);
        buf.push(4);
        buf.push(5);
        buf.push(6);
        assert_eq!(buf.pop(), Some(3));
        assert_eq!(buf.pop(), Some(4));
        assert_eq!(buf.pop(), Some(5));
        assert_eq!(buf.pop(), None);
    }
}
