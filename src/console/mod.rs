mod pl011;
mod printing;
mod write_buffer;

pub use pl011::{Pl011Regs, SERIAL};
#[cfg(test)]
pub use printing::print;
pub use printing::{println, write_args, write_str};
pub use write_buffer::WriteBuffer;
