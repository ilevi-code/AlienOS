use super::{bytes_reader::BytesReader, error::FdtParseError};

#[derive(Debug)]
pub enum InterruptType {
    Spi,
    Ppi,
}

#[derive(Debug)]
pub struct Interrupt {
    int_type: InterruptType,
    number: u32,
    flags: u16,
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
        let int_type = match int_type {
            0 => InterruptType::Spi,
            1 => InterruptType::Ppi,
            other => return Some(Err(FdtParseError::UnknownInterruptType(other))),
        };
        let flags: u16 = match flags.try_into() {
            Ok(flags) => flags,
            Err(_) => return Some(Err(FdtParseError::InvalidInterruptFlags(flags))),
        };
        Some(Ok(Interrupt {
            int_type,
            number,
            flags,
        }))
    }
}
