#![no_std]
#![no_main]

mod console;
mod kalloc;
mod mmu;
mod panic_handler;
mod dtb;

use dtb::DeviceTreeBlob;
use mmu::BasicTranslationTable;

#[no_mangle]
pub unsafe extern "C" fn main(_dtb: *mut DeviceTreeBlob, _bootstrap_table: *mut BasicTranslationTable) -> ! {
    kalloc::init();
    // let translate_table = TranslationTable::new(unsafe { &mut (*bootstrap_table) as &mut TranslationTable});
    console::write("hello\n");
    panic!("kernel has reached it's end");
}
