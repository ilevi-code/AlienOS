#[repr(C)]
pub(super) struct GicDispatcher {
    pub ctlr: u32,
    _align: [u8; 252],
    pub isenabler: [u32; 7],
}

impl GicDispatcher {
    pub(super) fn enable_forarding(&mut self) {
        self.ctlr = 1;
    }

    pub(super) fn enable_interrupt(&mut self, interrupt_number: usize) {
        let index = interrupt_number / 32;
        let shift = interrupt_number % 32;
        self.isenabler[index] |= 1 << shift;
    }
}
