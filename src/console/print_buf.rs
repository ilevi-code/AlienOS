use core::{cmp::Ordering, mem::size_of};

pub(super) const ENTRY_MAX_LENGTH: usize = 256;

pub(super) struct PrintBuf<const SIZE: usize> {
    buffer: [u8; SIZE],
    start: usize,
    size: usize,
}

impl<const SIZE: usize> PrintBuf<SIZE> {
    pub(super) const fn new() -> Self {
        Self {
            buffer: [0; SIZE],
            start: 0,
            size: 0,
        }
    }

    fn end(&self) -> usize {
        (self.start + self.size) % SIZE
    }

    // Write `data` into the buffer.
    // If the slice is bigger than `ENTRY_MAX_LENGTH`, it is truncated.
    pub fn push(&mut self, data: &[u8]) {
        let n = core::cmp::min(data.len(), ENTRY_MAX_LENGTH);

        let padding = Self::padding_needed(n);
        let total_size = size_of::<u32>() + n + padding;

        // Drop oldest until enough space
        while total_size > self.space_available() {
            self.consume_entry();
        }

        self.write_bytes(&(n as u32).to_le_bytes());
        self.write_bytes(&data[..n]);

        self.size += padding;
    }

    fn padding_needed(len: usize) -> usize {
        (4 - (len % 4)) % 4
    }

    fn space_available(&self) -> usize {
        SIZE - self.size
    }

    fn consume_entry(&mut self) {
        let mut len_bytes = [0u8; 4];
        self.read_bytes(&mut len_bytes);
        let entry_len = u32::from_le_bytes(len_bytes) as usize;
        let padding = Self::padding_needed(entry_len);
        self.consume_bytes(entry_len + padding);
    }

    fn consume_bytes(&mut self, len: usize) {
        let until_end = SIZE - self.start;
        match len.cmp(&until_end) {
            Ordering::Less => self.start += len,
            Ordering::Equal => self.start = 0,
            Ordering::Greater => self.start = len - until_end,
        }
        self.size -= len;
    }

    fn write_bytes(&mut self, data: &[u8]) {
        let index = self.end();
        let first_part = core::cmp::min(data.len(), SIZE - index);
        self.buffer[index..index + first_part].copy_from_slice(&data[..first_part]);

        // wrap around needed
        if first_part < data.len() {
            let second_part = data.len() - first_part;
            self.buffer[..second_part].copy_from_slice(&data[first_part..]);
        }
        self.size += data.len();
    }

    fn read_bytes(&mut self, out: &mut [u8]) {
        let first_part = core::cmp::min(out.len(), SIZE - self.start);
        out[..first_part].copy_from_slice(&self.buffer[self.start..self.start + first_part]);

        if first_part == out.len() {
            self.start += first_part;
        } else {
            let second_part = out.len() - first_part;
            out[first_part..].copy_from_slice(&self.buffer[..second_part]);
            self.start = second_part;
        }
        self.size -= out.len();
    }

    pub fn pop_into(&mut self, data: &mut [u8]) -> usize {
        if self.size == 0 {
            return 0;
        }

        // Read length
        let mut len_bytes = [0u8; 4];
        self.read_bytes(&mut len_bytes);
        let entry_len = u32::from_le_bytes(len_bytes) as usize;

        // Read data
        let read_size = if data.len() < entry_len {
            data.len()
        } else {
            entry_len
        };
        self.read_bytes(&mut data[..read_size]);
        let padding = Self::padding_needed(entry_len);
        self.consume_bytes(entry_len - read_size + padding);

        read_size
    }
}

#[test_case]
fn padding() {
    let padding_needed = PrintBuf::<1024>::padding_needed;
    assert_eq!(padding_needed(0), 0);
    assert_eq!(padding_needed(1), 3);
    assert_eq!(padding_needed(2), 2);
    assert_eq!(padding_needed(3), 1);
    assert_eq!(padding_needed(4), 0);
    assert_eq!(padding_needed(5), 3);
}

#[test_case]
fn layout() {
    let mut buf = PrintBuf::<256>::new();
    buf.push(b"abcd");
    buf.push(b"Hello");
    buf.push(b"Worlding blablaa");

    assert_eq!(
        &buf.buffer[..buf.size],
        [
            4, 0, 0, 0, 97, 98, 99, 100, 5, 0, 0, 0, 72, 101, 108, 108, 111, 0, 0, 0, 16, 0, 0, 0,
            87, 111, 114, 108, 100, 105, 110, 103, 32, 98, 108, 97, 98, 108, 97, 97,
        ]
    );
}

#[test_case]
fn popping() {
    let mut buf = PrintBuf::<256>::new();
    buf.push(b"abcd");
    buf.push(b"Hello");
    // Uncommenting this fails the test
    buf.push(b"Worlding blablaa");

    let mut entry = [0u8; ENTRY_MAX_LENGTH];
    let n = buf.pop_into(&mut entry);
    assert_eq!(&entry[..n], b"abcd");
    let n = buf.pop_into(&mut entry);
    assert_eq!(&entry[..n], b"Hello");
    let n = buf.pop_into(&mut entry);
    assert_eq!(&entry[..n], b"Worlding blablaa");
}

#[test_case]
fn wrap_around_and_discard() {
    let mut buf = PrintBuf::<28>::new();
    buf.push(b"12345"); // takes 4+8 bytes
    buf.push(b"abcde"); // takes 4+8 bytes

    // 4 bytes left, should reclaim first entry and also wrap-around
    buf.push(b"wazzap");

    assert_eq!(buf.start, 12);
    assert_eq!(buf.size, 24);
    let mut entry = [0u8; ENTRY_MAX_LENGTH];
    let n = buf.pop_into(&mut entry);
    assert_eq!(&entry[..n], b"abcde");
    let n = buf.pop_into(&mut entry);
    assert_eq!(&entry[..n], b"wazzap");
}
