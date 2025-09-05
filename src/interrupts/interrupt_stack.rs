use core::{arch::asm, ptr::NonNull};

use static_assertions::const_assert;

use crate::{
    arch::{set_stack_for_pe, PeMode},
    error::Result,
    heap,
    memory_model::{self, BOOT_STACK_SIZE},
    mmu::{PagePerm, TranslationTable, SMALL_PAGE_SIZE},
};

#[repr(align(4096))]
struct InterruptStack(#[allow(unused)] [u8; SMALL_PAGE_SIZE]);

const_assert!(align_of::<InterruptStack>() == SMALL_PAGE_SIZE);

fn alloc_stack() -> Result<NonNull<InterruptStack>> {
    let stack = heap::alloc::<InterruptStack>()?;
    let phys = memory_model::virt_to_phys(stack as *mut ());
    let ptr = TranslationTable::get_kernel().map_stack(
        phys,
        size_of::<InterruptStack>(),
        PagePerm::KernOnly,
    )?;
    Ok(ptr.cast::<InterruptStack>())
}

pub fn setup_interrupt_stacks(mode: PeMode) -> Result<()> {
    let stack = alloc_stack()?;
    // Stack grows down, so we need to start from highest address
    let stack_top = stack.addr().get() + size_of::<InterruptStack>();
    set_stack_for_pe(stack_top, mode);
    Ok(())
}

pub fn dup_stack(stack_top: usize) -> Result<()> {
    let new_stack = alloc_stack()?.as_ptr();
    let stack_low_addr = stack_top - BOOT_STACK_SIZE;
    const_assert!(size_of::<InterruptStack>() == BOOT_STACK_SIZE);

    unsafe {
        (new_stack as *mut u8)
            .copy_from_nonoverlapping(stack_low_addr as *const u8, BOOT_STACK_SIZE);
    }

    // Stack grows down, so we need to start from highest address
    let new_stack_top = new_stack.addr() + size_of::<InterruptStack>();
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
