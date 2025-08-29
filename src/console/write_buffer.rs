use core::cmp::min;
use core::fmt;

use super::Pl011Regs;

pub struct WriteBuffer<'serial> {
    buffer: [u8; 256],
    used: usize,
    serial: &'serial mut Pl011Regs,
}

impl<'serial> WriteBuffer<'serial> {
    pub fn new(serial: &'serial mut Pl011Regs) -> Self {
        WriteBuffer {
            buffer: [0; 256],
            used: 0,
            serial,
        }
    }

    pub fn flush(&mut self) {
        if self.used != 0 {
            Self::write_bytes(self.serial, &self.buffer);
            self.used = 0;
        }
    }

    pub fn write_bytes(serial: &mut Pl011Regs, bytes: &[u8]) {
        for byte in bytes {
            serial.set_data(*byte as u32);
        }
    }
}

impl<'serial> fmt::Write for WriteBuffer<'serial> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let remaining_buf = &mut self.buffer[self.used..];
        let raw = s.as_bytes();
        let copy_size = min(raw.len(), remaining_buf.len());

        remaining_buf[..copy_size].copy_from_slice(&raw[..copy_size]);

        if copy_size < raw.len() {
            self.flush();
            Self::write_bytes(self.serial, &raw[copy_size..]);
        } else {
            self.used += copy_size;
        }

        Ok(())
    }
}
