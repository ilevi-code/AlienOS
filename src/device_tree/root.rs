use crate::alloc::Box;

use super::{
    consume::Consume,
    error::FdtParseError,
    interrupt_controller::InterruptController,
    memory::Memory,
    parse::Parse,
    pl011::Pl011,
    timer::Timer,
    tokens::{Token, TokenReader},
};

#[derive(Debug)]
pub struct TreeRoot {
    pub memory: Box<Memory>,
    pub timer: Box<Timer>,
    pub interrupt_controller: Box<InterruptController>,
    pub pl011: Box<Pl011>,
}

impl<'data> Parse<'data> for TreeRoot {
    fn parse(parser: &mut TokenReader<'data>) -> Result<Self, FdtParseError<'data>> {
        let mut memory: Option<Box<Memory>> = None;
        let mut timer: Option<Box<Timer>> = None;
        let mut interrupt_controller: Option<Box<InterruptController>> = None;
        let mut pl011: Option<Box<Pl011>> = None;
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
                "memory" => memory = Some(Box::<Memory>::parse(parser)?),
                "timer" => timer = Some(Box::<Timer>::parse(parser)?),
                "intc" => interrupt_controller = Some(Box::<InterruptController>::parse(parser)?),
                "pl011" => pl011 = Some(Box::<Pl011>::parse(parser)?),
                _ => _ = Consume::parse(parser)?,
            };
        }
        let memory = memory.ok_or(FdtParseError::MissingField("/", "memory"))?;
        let timer = timer.ok_or(FdtParseError::MissingField("/", "timer"))?;
        let interrupt_controller =
            interrupt_controller.ok_or(FdtParseError::MissingField("/", "intc"))?;
        let pl011 = pl011.ok_or(FdtParseError::MissingField("/", "pl011"))?;
        Ok(TreeRoot {
            memory,
            timer,
            interrupt_controller,
            pl011,
        })
    }
}
