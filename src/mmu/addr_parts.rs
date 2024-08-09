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

impl From<usize> for AddrParts {
    fn from(virt: usize) -> Self {
        Self {
            l1_index: virt >> 20,
            l2_index: (virt >> 12) & 0xff,
            page_offset: virt & 0xfff,
        }
    }
}
