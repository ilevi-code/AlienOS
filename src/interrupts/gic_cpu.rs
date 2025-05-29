#[repr(C)]
pub(super) struct GicCpu {
    pub ctlr: u32,
    pub pmr: u32,
    _bpr: u32,
    pub iar: u32,
    pub eoir: u32,
}

impl GicCpu {
    pub(super) const ALLOW_ALL: u32 = 0xf0;

    pub(super) fn enable_singaling_to_cpu(&mut self) {
        self.ctlr = 1;
    }

    pub(super) fn set_prio_mask(&mut self, mask: u32) {
        self.pmr = mask;
    }

    pub(super) fn current_interrupt_number(&self) -> u32 {
        self.iar
    }

    pub(super) fn signal_end(&mut self, interrupt_number: u32) {
        self.eoir = interrupt_number;
    }
}
