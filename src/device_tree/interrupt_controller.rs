use crate::device_tree::consume::Consume;

use super::{
    bytes_reader::BytesReader,
    error::FdtParseError,
    parse::Parse,
    tokens::{Token, TokenReader},
};

#[derive(Debug)]
pub(crate) struct InterruptController {
    pub(crate) distributor: usize,
    pub(crate) cpu_interface: usize,
}

impl<'data> Parse<'data> for InterruptController {
    fn parse(parser: &mut TokenReader<'data>) -> Result<Self, FdtParseError<'data>> {
        let mut distributor: Option<usize> = None;
        let mut cpu_interface: Option<usize> = None;
        loop {
            let Some(node) = parser.read_token() else {
                return Err(FdtParseError::MissingTokenEnd {
                    current_type: "intc",
                });
            };
            let node = node?;
            let (name, value) = match node {
                Token::BeginNode(name) => {
                    Consume::parse(parser)?;
                    continue;
                }
                Token::EndNode => break,
                Token::Property { name, value } => (name, value),
                Token::Nop => continue,
                Token::End => todo!(),
            };
            match name {
                "reg" => {
                    let mut reader = BytesReader::from_bytes(value);
                    let distributor_start = reader
                        .read_u64()
                        .ok_or(FdtParseError::ValueTooShort("memory", "reg"))?
                        as usize;
                    distributor = Some(distributor_start);
                    let _distributor_size = reader
                        .read_u64()
                        .ok_or(FdtParseError::ValueTooShort("memory", "reg"))?
                        as usize;
                    let cpu_interface_start = reader
                        .read_u64()
                        .ok_or(FdtParseError::ValueTooShort("memory", "reg"))?
                        as usize;
                    cpu_interface = Some(cpu_interface_start);
                    let _cpu_interface_size = reader
                        .read_u64()
                        .ok_or(FdtParseError::ValueTooShort("memory", "reg"))?
                        as usize;
                }
                _ => (),
            };
        }
        let distributor = distributor.ok_or(FdtParseError::MissingField("memory", "reg"))?;
        let cpu_interface = cpu_interface.ok_or(FdtParseError::MissingField("memory", "reg"))?;
        Ok(Self {
            distributor,
            cpu_interface,
        })
    }
}
