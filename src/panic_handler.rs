use crate::console;
use core::panic::PanicInfo;

#[panic_handler]
unsafe fn panic_handler(panic_info: &PanicInfo) -> ! {
    let res = console::write_args(format_args!("{}\n", panic_info));
    if res.is_err() {
        console::write("\n--- error formatting panic info ---\n");
    }
    loop {}
}
