use core::arch::asm;

pub fn get_ttbr0() -> usize {
    let table;
    unsafe {
        asm!("MRC p15, 0, {table}, c2, c0, 0", table = out(reg) table);
    }
    table
}

pub fn set_ttbr0(table: usize) {
    unsafe {
        asm!("MCR p15, 0, {table}, c2, c0, 0", table = in(reg) table);
    }
}

pub fn get_ttbr1() -> usize {
    let table;
    unsafe {
        asm!("MRC p15, 0, {table}, c2, c0, 1", table = out(reg) table);
    }
    table
}

pub fn set_ttbr1(table: usize) {
    unsafe {
        asm!("MCR p15, 0, {table}, c2, c0, 1", table = in(reg) table);
    }
}

pub fn get_ttbcr() -> usize {
    let value;
    unsafe {
        asm!("MRC p15, 0, {}, c2, c0, 2", out(reg) value);
    }
    value
}

pub fn set_ttbcr(value: usize) {
    unsafe {
        asm!("MCR p15, 0, {}, c2, c0, 2", in(reg) value);
    }
}

pub fn get_cpsr() -> usize {
    let value;
    unsafe {
        asm!("MRS {}, CPSR", out(reg) value);
    }
    value
}

pub enum PeMode {
    User = 0b0000,
    Fiq = 0b0001,
    Irq = 0b0010,
    Supervisor = 0b0011,
    Abort = 0b0111,
}

pub fn set_stack_for_pe(stack: usize, mode: PeMode) {
    let cpsr = get_cpsr();
    let request_cpsr = (cpsr & !0xf) | mode as usize;
    unsafe {
        core::arch::asm!(
            "msr CPSR_c, {request_cpsr}",
            "mov sp, {stack}",
            "msr CPSR_c, {cpsr}",
            stack = in(reg) stack,
            request_cpsr = in(reg) request_cpsr,
            cpsr = in(reg) cpsr,
        );
    }
}
