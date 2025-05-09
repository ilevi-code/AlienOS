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
