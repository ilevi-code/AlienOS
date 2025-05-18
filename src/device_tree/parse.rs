use super::{error::FdtParseError, tokens::TokenReader};

pub(crate) trait Parse<'data>: Sized {
    fn parse(parser: &mut TokenReader<'data>) -> Result<Self, FdtParseError<'data>>;
}
