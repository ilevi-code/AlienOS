use core::panic::PanicInfo;

use crate::console;
use crate::semihosting;

#[panic_handler]
unsafe fn panic_handler(panic_info: &PanicInfo) -> ! {
    let res = console::write_args(format_args!("{}\n", panic_info));
    if res.is_err() {
        console::write_str("\n--- error formatting panic info ---\n");
    }
    semihosting::shutdown(1)
}
