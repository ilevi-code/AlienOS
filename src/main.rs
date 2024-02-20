#![no_std]
#![no_main]

mod console;
mod dtb;
mod kalloc;
mod mmu;
mod panic_handler;

use dtb::DeviceTreeBlob;
use mmu::BasicTranslationTable;

#[no_mangle]
pub unsafe extern "C" fn main(
    _dtb: *mut DeviceTreeBlob,
    bootstrap_table: *mut BasicTranslationTable,
) -> ! {
    kalloc::init();
    init_mmu_fine_grained(bootstrap_table);
    // let translate_table = TranslationTable::new(unsafe { &mut (*bootstrap_table) as &mut TranslationTable});
    console::write("hello\n");
    panic!("kernel has reached it's end");
}

fn init_mmu_fine_grained(bootstrap_table: *mut BasicTranslationTable) {
    let bootstrap_table = unsafe { &mut *bootstrap_table };
    let kern_location = mmu::get_kernel_location();
    let kern_phys = bootstrap_table
        .virt_to_phys(kern_location.start)
        .expect("Kernel should be mapped");
    let mut builder = mmu::TranslationTableBuilder::new(bootstrap_table)
        .expect("Base MMU builder create should succeed");
    builder.prepare_map(kern_location.start, kern_phys, kern_location.len());
    builder.apply();
}
