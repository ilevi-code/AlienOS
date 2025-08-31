use core::cmp::min;
use core::fmt;

use crate::console::print_buf::ENTRY_MAX_LENGTH;

pub(super) struct FmtBuffer {
    buffer: [u8; ENTRY_MAX_LENGTH],
    used: usize,
}

impl FmtBuffer {
    pub(super) fn new() -> Self {
        FmtBuffer {
            buffer: [0; ENTRY_MAX_LENGTH],
            used: 0,
        }
    }

    pub(super) fn as_bytes(&self) -> &[u8] {
        &self.buffer[..self.used]
    }
}

impl fmt::Write for FmtBuffer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let remaining_buf = &mut self.buffer[self.used..];
        let raw = s.as_bytes();
        // Truncate entries if they are too long
        let copy_size = min(raw.len(), remaining_buf.len());

        remaining_buf[..copy_size].copy_from_slice(&raw[..copy_size]);
        self.used += copy_size;
        Ok(())
    }
}
