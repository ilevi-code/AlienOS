use super::{
    bytes_reader::BytesReader,
    error::FdtParseError,
    parse::Parse,
    tokens::{Token, TokenReader},
};

#[derive(Debug)]
pub(crate) struct Clock {
    pub(crate) frequency: u32,
}

impl<'data> Parse<'data> for Clock {
    fn parse(parser: &mut TokenReader<'data>) -> Result<Self, FdtParseError<'data>> {
        let mut frequency: Option<u32> = None;
        loop {
            let Some(node) = parser.read_token() else {
                return Err(FdtParseError::MissingTokenEnd {
                    current_type: "apb-pclk",
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
            if name == "clock-frequency" {
                let mut reader = BytesReader::from_bytes(value);
                frequency = Some(reader
                    .read_u32()
                    .ok_or(FdtParseError::ValueTooShort("apb-pclk", "clock-frequency"))?);
            }
        }
        let frequency = frequency.ok_or(FdtParseError::MissingField("apb-pclk", "frequency"))?;
        Ok(Clock { frequency })
    }
}
