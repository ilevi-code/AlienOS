use super::error::FdtParseError;
use super::flattened_header::FlattenedHeader;
use super::parse::Parse;
use super::root::TreeRoot;
use super::tokens::{Token, TokenReader};

pub(crate) struct DeviceTree<'a> {
    dtb: &'a FlattenedHeader,
}

impl<'a> DeviceTree<'a> {
    pub(crate) fn from(address: *mut u8) -> DeviceTree<'a> {
        let dtb = unsafe { &*(address as *const FlattenedHeader) };
        DeviceTree { dtb }
    }
}

impl<'a> DeviceTree<'a> {
    pub(crate) fn parse_root(&self) -> Result<TreeRoot, FdtParseError<'a>> {
        self.parse_node_type::<TreeRoot>("")
    }

    #[allow(private_bounds)]
    pub(crate) fn parse_node_type<T: Parse<'a>>(
        &self,
        node_type: &str,
    ) -> Result<T, FdtParseError<'a>> {
        let strings = self.dtb.strings()?;
        let structs = self.dtb.structs()?;
        let mut parser: TokenReader<'a> = TokenReader::new(structs, strings);
        while let Some(token) = parser.read_token() {
            let token = token?;
            let found_type = match token {
                Token::BeginNode(node_type) => node_type,
                Token::End => break,
                _ => continue,
            };
            let found_type = found_type.split('@').next().unwrap();
            if found_type.starts_with(node_type) {
                return T::parse(&mut parser);
            }
        }
        Err(FdtParseError::NotFound)
    }

    pub(crate) fn len(&self) -> usize {
        self.dtb.len()
    }
}
