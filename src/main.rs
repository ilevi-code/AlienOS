#![no_std]
#![no_main]

mod console;
mod dtb;
mod kalloc;
mod mmu;
mod panic_handler;
mod step_range;

use dtb::DeviceTreeBlob;
use mmu::{virt_to_phys, EntryPtr, PagePerm};

#[no_mangle]
pub unsafe extern "C" fn main(_dtb: *mut DeviceTreeBlob, bootstrap_table: EntryPtr) -> ! {
    kalloc::init();
    init_mmu_fine_grained(bootstrap_table);
    // let translate_table = TranslationTable::new(unsafe { &mut (*bootstrap_table) as &mut TranslationTable});
    console::write("hello\n");
    panic!("kernel has reached it's end");
}

fn init_mmu_fine_grained(bootstrap_table: EntryPtr) {
    let kern_location = mmu::get_kernel_location();
    let kern_phys =
        virt_to_phys(bootstrap_table, kern_location.start).expect("Kernel should be mapped");
    let mut builder =
        mmu::TranslationTableBuilder::new().expect("Base MMU builder create should succeed");
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
