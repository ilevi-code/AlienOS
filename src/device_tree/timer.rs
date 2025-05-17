use super::{
    bytes_reader::BytesReader,
    error::FdtParseError,
    interrupts::{Interrupt, InterruptIterator},
    tokens::{Parse, Token, TokenReader},
};

#[derive(Debug)]
pub(super) struct Timer {
    virt_timer_interrupt: Interrupt,
}

impl<'data> Parse<'data> for Timer {
    fn parse(parser: &mut TokenReader<'data>) -> Result<Self, FdtParseError<'data>> {
        let mut virt_timer_interrupt: Option<Interrupt> = None;
        loop {
            let Some(node) = parser.read_token() else {
                return Err(FdtParseError::MissingTokenEnd {
                    current_type: "timer",
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
                    let _secure = reader
                        .next()
                        .ok_or(FdtParseError::ValueTooShort("timer", "interrupts"))??;
                    let _non_secure = reader
                        .next()
                        .ok_or(FdtParseError::ValueTooShort("timer", "interrupts"))??;
                    let virt = reader
                        .next()
                        .ok_or(FdtParseError::ValueTooShort("timer", "interrupts"))??;
                    virt_timer_interrupt = Some(virt);
                }
                _ => (),
            };
        }
        let virt_timer_interrupt =
            virt_timer_interrupt.ok_or(FdtParseError::MissingField("timer", "interrupts"))?;
        Ok(Timer {
            virt_timer_interrupt,
        })
    }
}
