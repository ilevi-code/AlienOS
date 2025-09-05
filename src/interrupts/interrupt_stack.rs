use core::arch::asm;

use static_assertions::const_assert;

use crate::{error::Result, heap, memory_model::BOOT_STACK_SIZE, mmu::SMALL_PAGE_SIZE};

#[repr(align(4096))]
struct InterruptStack(#[allow(unused)] [u8; SMALL_PAGE_SIZE]);

const_assert!(align_of::<InterruptStack>() == SMALL_PAGE_SIZE);

pub fn dup_stack(stack_top: usize) -> Result<()> {
    let new_stack = heap::alloc::<InterruptStack>()?;
    let stack_low_addr = stack_top - BOOT_STACK_SIZE;
    const_assert!(size_of::<InterruptStack>() == BOOT_STACK_SIZE);

    let new_stack_top = new_stack.addr() + size_of::<InterruptStack>();
    unsafe {
        (new_stack as *mut u8)
            .copy_from_nonoverlapping(stack_low_addr as *const u8, BOOT_STACK_SIZE);
    }
    unsafe {
        // $sp = new_stack_top - (offset_in_current_stack);
        asm!(
            "sub {tmp}, {stack_top}, sp", // calculate offset in current stack
            "sub {tmp}, {new_stack_top}, {tmp}", // apply the offset to current stack
            "mov sp, {tmp}",
            new_stack_top = in(reg) new_stack_top,
            stack_top = in(reg) stack_top,
            tmp = out(reg) _,
        );
    }

    Ok(())
}
