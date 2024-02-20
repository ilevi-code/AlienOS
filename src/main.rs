#![no_std]
#![no_main]

mod console;
mod dtb;
mod kalloc;
mod mmu;
mod panic_handler;

use dtb::DeviceTreeBlob;
use mmu::{EntryPtr, virt_to_phys};

#[no_mangle]
pub unsafe extern "C" fn main(
    _dtb: *mut DeviceTreeBlob,
    bootstrap_table: EntryPtr,
) -> ! {
    kalloc::init();
    init_mmu_fine_grained(bootstrap_table);
    // let translate_table = TranslationTable::new(unsafe { &mut (*bootstrap_table) as &mut TranslationTable});
    console::write("hello\n");
    panic!("kernel has reached it's end");
}

fn init_mmu_fine_grained(bootstrap_table: EntryPtr) {
    let kern_location = mmu::get_kernel_location();
    let kern_phys = virt_to_phys(bootstrap_table, kern_location.start)
        .expect("Kernel should be mapped");
    let mut builder = mmu::TranslationTableBuilder::new(bootstrap_table)
        .expect("Base MMU builder create should succeed");
    builder.prepare_map(kern_location.start, kern_phys, kern_location.len());
    builder.apply();
}
