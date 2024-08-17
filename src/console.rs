use core::cmp::min;
use core::fmt;
use core::sync::atomic::{AtomicUsize, Ordering};

pub(crate) static UART: AtomicUsize = AtomicUsize::new(0x9000000usize);

pub fn write(s: &str) {
    let uart0 = UART.load(Ordering::Acquire) as *mut u8;
    for byte in s.bytes() {
        unsafe {
            uart0.write_volatile(byte);
        }
    }
}

pub struct WriteBuffer {
    buffer: [u8; 256],
    used: usize,
}

impl WriteBuffer {
    pub fn new() -> Self {
        WriteBuffer {
            buffer: [0; 256],
            used: 0,
        }
    }

    fn flush(&mut self) {
        if self.used != 0 {
            write(unsafe { core::str::from_utf8_unchecked(&self.buffer[..self.used]) });
            self.used = 0;
        }
    }
}

impl fmt::Write for WriteBuffer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let remaining_buf = &mut self.buffer[self.used..];
        let raw = s.as_bytes();
        let copy_size = min(raw.len(), remaining_buf.len());

        remaining_buf[..copy_size].copy_from_slice(&raw[..copy_size]);

        if copy_size < raw.len() {
            self.flush();
            write(unsafe { core::str::from_utf8_unchecked(&raw[copy_size..]) });
        } else {
            self.used += copy_size;
        }

        Ok(())
    }
}

pub fn write_args(args: fmt::Arguments) -> Result<(), fmt::Error> {
    let mut buf = WriteBuffer::new();
    fmt::write(&mut buf, args)?;
    buf.flush();
    Ok(())
}

macro_rules! println {
    () => {
        $crate::console::write("\n")
    };
    ($($arg:tt)*) => {{
        $crate::console::write_args(format_args!($($arg)*)).unwrap();
        $crate::console::write("\n");
    }};
}

macro_rules! print {
    () => {};
    ($($arg:tt)*) => {{
        $crate::console::write_args(format_args!($($arg)*)).unwrap();
    }};
}

pub(crate) use print;
pub(crate) use println;
