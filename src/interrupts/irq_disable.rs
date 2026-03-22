use crate::per_cpu;

per_cpu!(INTERRUPT_DISABLE_DEPTH: u32 = 0);

pub fn without_irq<Ret, F: FnOnce() -> Ret>(f: F) -> Ret {
    unsafe { core::arch::asm!("CPSID i") };
    {
        *INTERRUPT_DISABLE_DEPTH.borrow_mut() += 1;
    }
    let ret = f();
    let depth_local = {
        let mut depth = INTERRUPT_DISABLE_DEPTH.borrow_mut();
        let old_value = *depth;
        *depth -= 1;
        old_value - 1
    };
    if depth_local == 0 {
        unsafe { core::arch::asm!("CPSIE i") };
    }
    ret
}

pub fn irq_state_save<Ret, F: FnOnce() -> Ret>(f: F) -> Ret {
    let depth = INTERRUPT_DISABLE_DEPTH.get();
    let ret = f();
    INTERRUPT_DISABLE_DEPTH.replace(depth);
    ret
}
