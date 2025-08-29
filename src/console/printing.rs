use core::fmt::{self, Write};

pub fn write_args(args: fmt::Arguments) -> Result<(), fmt::Error> {
    let mut serial = SERIAL.lock();
    let mut buf = super::WriteBuffer::new(&mut serial);
    fmt::write(&mut buf, args)?;
    buf.flush();
    Ok(())
}

pub fn write_str(s: &str) {
    let mut serial = SERIAL.lock();
    let mut buf = super::WriteBuffer::new(&mut serial);
    buf.write_str(s).unwrap();
    buf.flush();
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::console::write_str("\n")
    };
    ($($arg:tt)*) => {{
        $crate::console::write_args(format_args!($($arg)*)).unwrap();
        $crate::console::write_str("\n");
    }};
}

#[cfg(test)]
#[macro_export]
macro_rules! print {
    () => {};
    ($($arg:tt)*) => {{
        $crate::console::write_args(format_args!($($arg)*)).unwrap();
    }};
}

#[cfg(test)]
pub use print;
pub use println;

use super::pl011::SERIAL;
