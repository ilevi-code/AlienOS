#[derive(Debug)]
pub enum FdtParseError<'a> {
    CorruptHeader,
    UnknownToken(u32),
    MissingTokenEnd { current_type: &'static str },
    MissingField(&'static str, &'static str),
    ValueTooShort(&'static str, &'static str),
    UnexpectedNode(&'a str),
    UnknownInterruptType(u32),
    InvalidInterruptFlags(u32),
}
