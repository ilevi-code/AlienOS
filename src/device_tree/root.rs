use super::{
    consume::Consume,
    error::FdtParseError,
    memory::Memory,
    timer::Timer,
    tokens::{Parse, Token, TokenReader},
};

#[derive(Debug)]
pub struct TreeRoot {
    memory: Memory,
    timer: Timer,
}

impl<'t, 'data: 't> Parse<'t, 'data> for TreeRoot {
    fn parse(parser: &'t mut TokenReader<'data>) -> Result<Self, FdtParseError<'data>> {
        let mut memory: Option<Memory> = None;
        let mut timer: Option<Timer> = None;
        loop {
            let Some(node) = parser.read_token() else {
                return Err(FdtParseError::MissingTokenEnd { current_type: "/" });
            };
            let node = node?;
            let node_name = match node {
                Token::BeginNode(name) => name,
                Token::EndNode => break,
                Token::Property { .. } => continue,
                Token::Nop => continue,
                Token::End => todo!(),
            };
            match node_name.split('@').next().unwrap() {
                "memory" => memory = Some(Memory::parse(parser)?),
                "timer" => timer = Some(Timer::parse(parser)?),
                _ => _ = Consume::parse(parser)?,
            };
        }
        let memory = memory.ok_or(FdtParseError::MissingField("/", "memory"))?;
        let timer = timer.ok_or(FdtParseError::MissingField("/", "timer"))?;
        Ok(TreeRoot { memory, timer })
    }
}
