use error::FdtParseError;
use flattened_header::FlattenedHeader;
use root::TreeRoot;
use tokens::{Parse, TokenReader};

mod bytes_reader;
mod consume;
mod error;
mod flattened_header;
mod interrupts;
mod memory;
mod root;
mod string_block;
mod timer;
mod tokens;

pub(crate) fn parse(dtb_address: usize) -> TreeRoot {
    let dtb = unsafe { &*(dtb_address as *const FlattenedHeader) };
    match parse_root(dtb) {
        Ok(root) => root,
        Err(e) => match e {
            FdtParseError::UnknownToken(node) => todo!(),
            FdtParseError::MissingTokenEnd { current_type } => todo!(),
            FdtParseError::MissingField(node, field) => todo!(),
            FdtParseError::ValueTooShort(node, field) => todo!(),
            FdtParseError::UnexpectedNode(node) => todo!(),
            FdtParseError::CorruptHeader => todo!(),
            FdtParseError::UnknownInterruptType(_) => todo!(),
            FdtParseError::InvalidInterruptFlags(_) => todo!(),
        },
    }
}

fn parse_root(dtb: &FlattenedHeader) -> Result<TreeRoot, FdtParseError> {
    let strings = dtb.strings()?;
    let structs = dtb.structs()?;
    let mut parser = TokenReader::new(structs, strings);
    let _root_begin = parser.read_token();
    // TODO check _root_begin
    Ok(TreeRoot::parse(&mut parser)?)
}
