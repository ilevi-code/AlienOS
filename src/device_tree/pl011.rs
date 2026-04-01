use super::{
    bytes_reader::BytesReader,
    error::FdtParseError,
    interrupts::{Interrupt, InterruptIterator},
    parse::Parse,
    tokens::{Token, TokenReader},
};
use crate::console::Pl011Regs;
use crate::phys::Phys;

#[derive(Debug)]
pub(crate) struct Pl011 {
    pub(crate) interrupt: Interrupt,
    pub(crate) address: Phys<Pl011Regs>,
}

impl<'data> Parse<'data> for Pl011 {
    fn parse(parser: &mut TokenReader<'data>) -> Result<Self, FdtParseError<'data>> {
        let mut interrupt: Option<Interrupt> = None;
        let mut address: Option<Phys<Pl011Regs>> = None;
        loop {
            let Some(node) = parser.read_token() else {
                return Err(FdtParseError::MissingTokenEnd {
                    current_type: "pl011",
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
                "interrupts" => {
                    let mut reader: InterruptIterator = BytesReader::from_bytes(value).into();
                    interrupt = Some(
                        reader
                            .next()
                            .ok_or(FdtParseError::ValueTooShort("pl011", "interrupts"))??,
                    );
                }
                "reg" => {
                    let mut reader = BytesReader::from_bytes(value);
                    let start = reader
                        .read_u64()
                        .ok_or(FdtParseError::ValueTooShort("pl011", "reg"))?
                        as usize;
                    let _size = reader
                        .read_u64()
                        .ok_or(FdtParseError::ValueTooShort("pl011", "reg"))?
                        as usize;
                    address = Some(Phys::<Pl011Regs>::from(start));
                }
                _ => (),
            };
        }
        let interrupt = interrupt.ok_or(FdtParseError::MissingField("pl011", "interrupt"))?;
        let address = address.ok_or(FdtParseError::MissingField("pl011", "address"))?;
        Ok(Pl011 { interrupt, address })
    }
}
