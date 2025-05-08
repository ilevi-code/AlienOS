// Offset an on address in the current table
pub(super) struct Offset(pub(super) usize);

pub(super) struct AddrParts {
    pub(super) l1_index: usize,
    pub(super) l2_index: usize,
    pub(super) page_offset: usize,
}

impl AddrParts {
    pub(super) fn section_offset(&self) -> usize {
        (self.l2_index << 12) + self.page_offset
    }
}

impl From<Offset> for AddrParts {
    fn from(offset: Offset) -> Self {
        Self {
            l1_index: offset.0 >> 20,
            l2_index: (offset.0 >> 12) & 0xff,
            page_offset: offset.0 & 0xfff,
        }
    }
}
