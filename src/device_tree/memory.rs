use super::{
    bytes_reader::BytesReader,
    error::FdtParseError,
    tokens::{Parse, Token, TokenReader},
};

#[derive(Debug)]
pub(crate) struct Memory {
    pub(crate) addresses: core::ops::Range<usize>,
}

impl<'data> Parse<'data> for Memory {
    fn parse(parser: &mut TokenReader<'data>) -> Result<Self, FdtParseError<'data>> {
        let mut addresses: Option<core::ops::Range<usize>> = None;
        loop {
            let Some(node) = parser.read_token() else {
                return Err(FdtParseError::MissingTokenEnd {
                    current_type: "memory",
                });
            };
            let node = node?;
            let (name, value) = match node {
                Token::BeginNode(name) => return Err(FdtParseError::UnexpectedNode(name)),
                Token::EndNode => break,
                Token::Property { name, value } => (name, value),
                Token::Nop => continue,
                Token::End => todo!(),
            };
            match name {
                "reg" => {
                    let mut reader = BytesReader::from_bytes(value);
                    let start = reader
                        .read_u64()
                        .ok_or(FdtParseError::ValueTooShort("memory", "reg"))?
                        as usize;
                    let size = reader
                        .read_u64()
                        .ok_or(FdtParseError::ValueTooShort("memory", "reg"))?
                        as usize;
                    addresses = Some(start..start + size);
                }
                _ => (),
            };
        }
        let addresses = addresses.ok_or(FdtParseError::MissingField("memory", "reg"))?;
        Ok(Memory { addresses })
    }
}
