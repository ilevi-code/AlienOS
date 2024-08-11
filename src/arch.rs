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
