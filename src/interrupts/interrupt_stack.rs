use core::{arch::asm, hint, ptr::NonNull};

use crate::{
    arch::{set_stack_for_pe, PeMode},
    error::{Error, Result},
    heap,
    mmu::{Page, PagePerm, TranslationTable},
    phys::Phys,
};

type Stack = Page;

fn alloc_stack() -> Result<NonNull<Stack>> {
    let stack = heap::alloc::<Stack>()?.cast_const();
    let phys = Phys::from_virt(Stack::as_slice_ptr(stack));
    let ptr = TranslationTable::get_kernel().map_stack(phys, PagePerm::KernOnly)?;
    Ok(ptr.cast::<Stack>())
}

pub fn setup_interrupt_stacks(mode: PeMode) -> Result<()> {
    let stack = alloc_stack()?;
    // Stack grows down, so we need to start from highest address
    let stack_top = stack.addr().get() + size_of::<Stack>();
    set_stack_for_pe(stack_top, mode);
    Ok(())
}

pub fn call_in_new_stack(f: extern "C" fn() -> !) -> Error {
    let new_stack = match alloc_stack() {
        Ok(stack) => stack.as_ptr(),
        Err(e) => return e,
    };

    // Stack grows down, so we need to start from highest address
    let stack_top = new_stack.addr() + size_of::<Stack>();

    unsafe {
        asm!(
            "mov sp, {stack_top}",
            "bx {f}",
            stack_top = in(reg) stack_top,
            f = in(reg) f,
        );
    }
    // SAFETY:
    // We branch to `f` without linking, so `f` can never return here.
    unsafe {
        hint::unreachable_unchecked();
    }
}
