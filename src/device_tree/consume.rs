use super::{
    error::FdtParseError,
    parse::Parse,
    tokens::{Token, TokenReader},
};

pub(super) struct Consume {}

impl<'data> Parse<'data> for Consume {
    fn parse(parser: &mut TokenReader<'data>) -> Result<Self, FdtParseError<'data>> {
        let mut ends_needed = 1;
        while ends_needed > 0 {
            let Some(Ok(token)) = parser.read_token() else {
                break;
            };
            match token {
                Token::BeginNode { .. } => ends_needed += 1,
                Token::EndNode => ends_needed -= 1,
                _ => (),
            };
        }
        Ok(Consume {})
    }
}
