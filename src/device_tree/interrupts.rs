use super::{bytes_reader::BytesReader, error::FdtParseError};
use crate::interrupts::Interrupt as RawInterrupt;

#[derive(Copy, Clone, Debug)]
pub struct Interrupt {
    pub interrupt: RawInterrupt,
    pub flags: u16,
}

pub(super) struct InterruptIterator<'a> {
    reader: BytesReader<'a>,
}

impl<'a> From<BytesReader<'a>> for InterruptIterator<'a> {
    fn from(reader: BytesReader<'a>) -> Self {
        Self { reader }
    }
}

impl<'a> Iterator for InterruptIterator<'a> {
    type Item = Result<Interrupt, FdtParseError<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        let int_type = self.reader.read_u32()?;
        let number = self.reader.read_u32()?;
        let flags = self.reader.read_u32()?;
        let interrupt = match int_type {
            0 => RawInterrupt::Spi(number as u8),
            1 => RawInterrupt::Ppi(number as u8),
            other => return Some(Err(FdtParseError::UnknownInterruptType(other))),
        };
        let flags: u16 = match flags.try_into() {
            Ok(flags) => flags,
            Err(_) => return Some(Err(FdtParseError::InvalidInterruptFlags(flags))),
        };
        Some(Ok(Interrupt { interrupt, flags }))
    }
}
