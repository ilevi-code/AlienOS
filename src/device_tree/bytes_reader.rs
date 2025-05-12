use core::ffi::CStr;

pub(super) struct BytesReader<'a> {
    block: &'a [u8],
    offset: usize,
}

impl<'this, 'data: 'this> BytesReader<'data> {
    pub(super) fn new(block: &'data [u8]) -> Self {
        Self { block, offset: 0 }
    }

    pub(super) fn from_bytes(slice: &'data [u8]) -> Self {
        Self {
            block: slice,
            offset: 0,
        }
    }

    pub(super) fn read_u32(&mut self) -> Option<u32> {
        let slice = self.read_bytes(4)?;
        Some(u32::from_be_bytes(slice[..].try_into().ok()?))
    }

    pub(super) fn read_u64(&mut self) -> Option<u32> {
        let upper = self.read_bytes(4)?;
        // We are 32-bit and do not support such high addresses
        if upper != [0; 4] {
            return None;
        }
        let lower = self.read_bytes(4)?;
        Some(u32::from_be_bytes(lower[..].try_into().ok()?))
    }

    pub(super) fn read_str(&'this mut self) -> Option<&'data str> {
        let rest_of_block = self.block.get(self.offset..)?;
        let s = CStr::from_bytes_until_nul(rest_of_block).ok()?;
        self.advance(s.count_bytes() + 1);
        Some(s.to_str().unwrap())
    }

    pub(super) fn read_bytes(&'this mut self, count: u32) -> Option<&'data [u8]> {
        let slice = self.block.get(self.offset..self.offset + count as usize)?;
        self.advance(slice.len());
        Some(slice)
    }

    pub(super) fn advance(&mut self, count: usize) {
        use crate::num::AlignUp;
        self.offset += count.align_up(core::mem::size_of::<u32>());
    }
}
