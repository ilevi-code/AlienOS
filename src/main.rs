#![reexport_test_harness_main = "test_main"]
#![feature(pointer_is_aligned_to, negative_impls, custom_test_frameworks)]
#![test_runner(crate::testing::test_runner)]
#![no_std]
#![no_main]

mod arch;
mod console;
mod dtb;
mod heap;
mod kernel_location;
mod memory_model;
mod mmu;
mod num;
mod panic_handler;
mod phys;
mod semihosting;
mod spinlock;
mod step_range;
mod testing;

use arch::get_ttbr0;
use dtb::DeviceTree;
use kernel_location::get_kernel_location;
use mmu::{PagePerm, TranslationTable};

// TODO parse from DTB
const RAM_SIZE: usize = 0x2000_0000;
const KERN_LINK: usize = 0xc000_0000;

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn main(_dtb: *mut DeviceTree, _bootstrap_table: usize) -> ! {
    #[cfg(test)]
    {
        test_main();
        semihosting::shutdown(0);
    }

    heap::init(get_kernel_location().end, KERN_LINK + RAM_SIZE);
    // TODO allocate enough space to copy and save the DeviceTree, before starting to do shit.

    init_mmu_fine_grained();
    panic!("kernel has reached it's end");
}

fn init_mmu_fine_grained() {
    let kern_location = get_kernel_location();
    let mut kern_table = TranslationTable::from_base(get_ttbr0());
    kern_table.unmap(kern_table.next_entry(kern_location.end).unwrap()..0xffff_ffff);
    // kern_table.map_device()
    // builder.apply();
}
