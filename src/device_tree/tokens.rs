use super::{bytes_reader::BytesReader, error::FdtParseError, string_block::StringBlock};

#[derive(Debug)]
pub(super) enum Token<'a> {
    BeginNode(&'a str),
    EndNode,
    Property { name: &'a str, value: &'a [u8] },
    Nop,
    End,
}

pub(super) struct TokenReader<'t> {
    structs: BytesReader<'t>,
    strings: StringBlock<'t>,
}

impl<'this, 't: 'this> TokenReader<'t> {
    pub(super) fn new(structs: BytesReader<'t>, strings: StringBlock<'t>) -> Self {
        Self { structs, strings }
    }

    pub(super) fn read_token(&'this mut self) -> Option<Result<Token<'t>, FdtParseError<'t>>> {
        let token = match self.structs.read_u32()? {
            1 => {
                let str = self.structs.read_str()?;
                // SAFETY:
                // Deduced lifetime is of &self, but it's actually the same as 'a
                let str = unsafe { &*(str as *const str) };
                Ok(Token::BeginNode(str))
            }
            3 => {
                let property_len = self.structs.read_u32()?;
                let name_offset = self.structs.read_u32()?;
                let value = self.structs.read_bytes(property_len)?;
                let value = unsafe { &*(value as *const [u8]) };
                let name = self.strings.at(name_offset)?;
                Ok(Token::Property { name, value })
            }
            2 => Ok(Token::EndNode),
            4 => Ok(Token::Nop),
            9 => Ok(Token::End),
            unknown => Err(FdtParseError::UnknownToken(unknown)),
        };
        Some(token)
    }
}
