#![no_std]
#![no_main]

mod console;
mod dtb;
mod kalloc;
mod mmu;
mod panic_handler;
mod step_range;

use dtb::DeviceTreeBlob;
use mmu::{PagePerm, TranslationTable};

// TODO parse from DTB
const RAM_SIZE: usize = 0x2000_0000;
const KERN_LINK: usize = 0xc000_0000;

#[no_mangle]
pub unsafe extern "C" fn main(_dtb: *mut DeviceTreeBlob, bootstrap_table: usize) -> ! {
    kalloc::init(mmu::get_kernel_location().end, KERN_LINK + RAM_SIZE);

    init_mmu_fine_grained(bootstrap_table);
    panic!("kernel has reached it's end");
}

fn init_mmu_fine_grained(bootstrap_table: usize) {
    let bootstrap_table = TranslationTable::from_base(bootstrap_table);
    let kern_location = mmu::get_kernel_location();
    let kern_phys = bootstrap_table
        .virt_to_phys(kern_location.start)
        .expect("Kernel should be mapped");
    let mut builder =
        mmu::TranslationTableBuilder::new().expect("Base MMU builder create should succeed");
    // TODO map whole sections, save allocation
    builder
        .map(0x0, 0x0000_0000, 0x1000_0000, PagePerm::KernOnly)
        .unwrap();
    // TODO map the bootloader, our stack is in this region
    builder
        .map(
            kern_location.start,
            kern_phys,
            kern_location.len(),
            PagePerm::KernOnly,
        )
        .expect("Mapping kernel should succeed");
    builder.apply();
}
