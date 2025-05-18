use crate::alloc::Box;

use super::{error::FdtParseError, tokens::TokenReader};

pub(crate) trait Parse<'data>: Sized {
    fn parse(parser: &mut TokenReader<'data>) -> Result<Self, FdtParseError<'data>>;
}

impl<'data, T> Parse<'data> for Box<T>
where
    T: Parse<'data>,
{
    fn parse(parser: &mut TokenReader<'data>) -> Result<Self, FdtParseError<'data>> {
        let value = T::parse(parser)?;
        Box::new(value).map_err(|_| FdtParseError::AllocError)
    }
}
