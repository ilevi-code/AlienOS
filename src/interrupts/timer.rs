pub struct VirtualCounter;

impl VirtualCounter {
    pub fn enable(&mut self) {
        // SAFETY: no memory changes, just enabling timter intrrupts.
        unsafe {
            // set the enable bit in CNTV_CTL
            core::arch::asm!(
                "MCR p15, 0, {tmp}, c14, c3, 1",
                tmp = in(reg) 1,
            );
        }
    }

    pub fn arm(&mut self, ticks: usize) {
        // SAFETY: no memory changes, just moving to a tick-counting register.
        unsafe {
            // arm CNTV_TVAL
            core::arch::asm!("MCR p15, 0, {}, c14, c3, 0", in(reg) ticks);
        }
    }

    /// Returns how many clock ticks there are in a second.
    pub fn frequency(&self) -> usize {
        let tick_frequency: usize;
        // SAFETY: reading from a register.
        unsafe {
            // Read from CNTFRQ
            core::arch::asm!("MRC p15, 0, {}, c14, c0, 0", out(reg) tick_frequency);
        }
        tick_frequency
    }
}
