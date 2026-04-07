use core::fmt::{self, Write};

use crate::{
    SpinLock, alloc::Arc, console::{
        print_buf::{ENTRY_MAX_LENGTH, PrintBuf as GenericPrintBuf},
        write_buffer::FmtBuffer,
    }, drivers::{CharDev, pl011::Pl011}, sys::AsUserBytes
};

pub static SERIAL: SpinLock<Option<Arc<Pl011>>> = SpinLock::new(None);

type PrintBuf = GenericPrintBuf<1024>;

static PRINT_BUF: SpinLock<PrintBuf> = SpinLock::new(PrintBuf::new());

fn console_flush_entris(serial: &Pl011) {
    loop {
        let mut line_buf = [0; ENTRY_MAX_LENGTH];
        let n = { PRINT_BUF.lock().pop_into(&mut line_buf) };
        if n == 0 {
            break;
        }
        // On write failure there is nothing for us to do
        let _ = serial.write(line_buf[..n].as_user_bytes());
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
        if let Some(serial_guard) = SERIAL.try_lock() {
            // If there is no serial line, the entries will be flushed a sucecessfull call
            if let Some(serial) = serial_guard.as_ref() {
                console_flush_entris(serial);
            }
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
