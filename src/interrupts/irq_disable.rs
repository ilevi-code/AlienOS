use crate::per_cpu;

per_cpu!(INTERRUPT_DISABLE_DEPTH: u32 = 0);

pub fn without_irq<F: FnOnce()>(f: F) {
    unsafe { core::arch::asm!("CPSID i") };
    {
        *INTERRUPT_DISABLE_DEPTH.borrow_mut() += 1;
    }
    f();
    let depth_local = {
        let mut depth = INTERRUPT_DISABLE_DEPTH.borrow_mut();
        let old_value = *depth;
        *depth -= 1;
        old_value - 1
    };
    if depth_local == 0 {
        unsafe { core::arch::asm!("CPSIE i") };
    }
}
