use core::sync::atomic::{AtomicPtr, Ordering};

#[repr(C)]
pub(super) struct Gicc {
    pub ctlr: u32,
    pub pmr: u32,
    _bpr: u32,
    pub iar: u32,
    pub eoir: u32,
}

impl Gicc {
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

#[repr(C)]
pub(super) struct Gicd {
    pub ctlr: u32,
    _align: [u8; 252],
    pub isenabler: [u32; 7],
}

impl Gicd {
    pub(super) fn enable_forarding(&mut self) {
        self.ctlr = 1;
    }

    pub(super) fn enable_interrupt(&mut self, interrupt_number: usize) {
        let index = interrupt_number / 32;
        let shift = interrupt_number % 32;
        self.isenabler[index] |= 1 << shift;
    }
}

pub(crate) static GICD: AtomicPtr<crate::gic::Gicd> =
    AtomicPtr::new(0x8000000 as *mut crate::gic::Gicd);
pub(crate) static GICC: AtomicPtr<crate::gic::Gicc> =
    AtomicPtr::new(0x8010000 as *mut crate::gic::Gicc);

pub(super) fn get_gicc() -> &'static mut Gicc {
    unsafe { &mut *GICC.load(Ordering::Acquire) }
}

pub(super) fn get_gicd() -> &'static mut Gicd {
    unsafe { &mut *GICD.load(Ordering::Acquire) }
}
