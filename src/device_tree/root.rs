use crate::alloc::Box;

use super::{
    consume::Consume,
    error::FdtParseError,
    interrupt_controller::InterruptController,
    parse::Parse,
    pl011::Pl011,
    timer::Timer,
    clock::Clock,
    tokens::{Token, TokenReader},
};

#[derive(Debug)]
pub struct TreeRoot {
    pub timer: Box<Timer>,
    pub interrupt_controller: Box<InterruptController>,
    pub pl011: Box<Pl011>,
    pub clock: Box<Clock>
}

impl<'data> Parse<'data> for TreeRoot {
    fn parse(parser: &mut TokenReader<'data>) -> Result<Self, FdtParseError<'data>> {
        let mut timer: Option<Box<Timer>> = None;
        let mut interrupt_controller: Option<Box<InterruptController>> = None;
        let mut pl011: Option<Box<Pl011>> = None;
        let mut clock: Option<Box<Clock>> = None;
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
                "timer" => timer = Some(Box::<Timer>::parse(parser)?),
                "intc" => interrupt_controller = Some(Box::<InterruptController>::parse(parser)?),
                "pl011" => pl011 = Some(Box::<Pl011>::parse(parser)?),
                "apb-pclk" => clock = Some(Box::<Clock>::parse(parser)?),
                _ => _ = Consume::parse(parser)?,
            };
        }
        let timer = timer.ok_or(FdtParseError::MissingField("/", "timer"))?;
        let interrupt_controller =
            interrupt_controller.ok_or(FdtParseError::MissingField("/", "intc"))?;
        let pl011 = pl011.ok_or(FdtParseError::MissingField("/", "pl011"))?;
        let clock = clock.ok_or(FdtParseError::MissingField("/", "apb-pclk"))?;
        Ok(TreeRoot {
            timer,
            interrupt_controller,
            pl011,
            clock,
        })
    }
}
