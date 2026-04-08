use core::arch::asm;

#[allow(unused)]
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

pub fn get_cpsr() -> usize {
    let value;
    unsafe {
        asm!("MRS {}, CPSR", out(reg) value);
    }
    value
}

#[inline(always)]
pub fn halt() {
    unsafe {
        asm!("WFI")
    }
}

#[allow(unused)]
pub enum PeMode {
    User = 0b10000,
    Fiq = 0b10001,
    Irq = 0b10010,
    Supervisor = 0b10011,
    Abort = 0b10111,
}

pub fn set_stack_for_pe(stack: usize, mode: PeMode) {
    let cpsr = get_cpsr();
    let request_cpsr = (cpsr & !0x1f) | mode as usize;
    unsafe {
        core::arch::asm!(
            "msr CPSR, {request_cpsr}",
            "mov sp, {stack}",
            "msr CPSR, {cpsr}",
            stack = in(reg) stack,
            request_cpsr = in(reg) request_cpsr,
            cpsr = in(reg) cpsr,
        );
    }
}

#[inline]
pub fn data_sync() {
    unsafe { asm!("dsb") }
}
