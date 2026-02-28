use core::fmt::{self, Write};

use crate::{
    console::{
        print_buf::{PrintBuf as GenericPrintBuf, ENTRY_MAX_LENGTH},
        write_buffer::FmtBuffer,
        Pl011Regs,
    },
    SpinLock,
};

use super::pl011::SERIAL;
type PrintBuf = GenericPrintBuf<1024>;

static PRINT_BUF: SpinLock<PrintBuf> = SpinLock::new(PrintBuf::new());

fn console_write(serial: &mut Pl011Regs, bytes: &[u8]) {
    for byte in bytes {
        serial.set_data(*byte as u32);
    }
}

fn console_flush_entris(serial: &mut Pl011Regs) {
    loop {
        let mut line_buf = [0; ENTRY_MAX_LENGTH];
        let n = { PRINT_BUF.lock().pop_into(&mut line_buf) };
        if n == 0 {
            break;
        }
        console_write(serial, &line_buf[..n]);
    }
}

pub fn write_args(args: fmt::Arguments, newline: bool) -> Result<(), fmt::Error> {
    let mut buf = FmtBuffer::new();
    fmt::write(&mut buf, args)?;
    if newline {
        buf.write_str("\n")?;
    }
    crate::interrupts::without_irq(|| {
        PRINT_BUF.lock().push(buf.as_bytes());
    });
    crate::interrupts::without_irq(|| {
        // If we fail to lock, this means that we are got interrupted while flusing.
        // We have already added submitted the formatted entry, so when we return from this
        // interrupt, we will notice the new entry.
        if let Some(mut serial) = SERIAL.try_lock() {
            console_flush_entris(&mut serial);
        }
    });
    Ok(())
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::console::print!("\n")
    };
    ($($arg:tt)*) => {{
        $crate::console::write_args(format_args!($($arg)*), true).unwrap();
    }};
}

#[macro_export]
macro_rules! print {
    () => {};
    ($($arg:tt)*) => {{
        $crate::console::write_args(format_args!($($arg)*), false).unwrap();
    }};
}

pub use print;
pub use println;
