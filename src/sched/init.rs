use crate::{arch, error::Result, heap, memory_model, phys::Phys};
use core::{
    arch::{asm, global_asm},
    ptr::addr_of,
};

use crate::{mmu, mmu::TranslationTable, spinlock::SpinLock};

global_asm!(".global init_code", "init_code:", "svc #0");

type Stack = [u8; 4096];

struct Proc {
    mappings: TranslationTable<'static>,
    stack: Phys<Stack>,
}

static PROCCES_LIST: SpinLock<Option<Proc>> = SpinLock::new(None);

extern "C" {
    static init_code: u8;
}

pub fn setup_init() -> Result<()> {
    // TODO fix leakes
    // let mut mappings = mmu::TranslationTable::new()?;
    let mut mappings = TranslationTable::get_kernel();
    let stack = heap::alloc::<Stack>()?;
    mappings.map(
        0x0,
        memory_model::virt_to_phys(addr_of!(init_code)).addr(),
        100,
        mmu::PagePerm::UserRo,
        true,
        true,
    )?;
    mappings.map(
        0x1000,
        stack.addr(),
        0x1000,
        mmu::PagePerm::UserRw,
        true,
        true,
    )?;
    // This faults. The current table is setup for 4GB of address space.
    // I need to map the current table to the higer addresses, the put it into a global, with a
    // mutex.
    // After that I can make the transition to 50-50 mapping.
    // Also I should map the console to higher addresses.
    // mappings.apply_user();
    // arch::set_ttbcr(arch::get_ttbcr() | 0x1);
    *PROCCES_LIST.lock() = Some(Proc { mappings, stack });

    unsafe {
        asm!(
            "mov sp, {}",
            "msr CPSR_c, #0x13",
            "bx {}",
            in(reg) 0x2000,
            in(reg) addr_of!(init_code) as usize & 0xfff,
        );
    }

    Ok(())
}
