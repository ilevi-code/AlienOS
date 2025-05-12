pub(super) struct StringBlock<'a> {
    block: &'a [u8],
}

impl<'this, 'data: 'this> StringBlock<'data> {
    pub(super) fn new(block: &'data [u8]) -> Self {
        Self { block }
    }

    pub(super) fn at(&'this self, index: u32) -> Option<&'data str> {
        let index = index as usize;
        core::ffi::CStr::from_bytes_until_nul(&self.block[index..])
            .ok()?
            .to_str()
            .ok()
    }
}
