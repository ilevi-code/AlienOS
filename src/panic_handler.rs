use core::panic::PanicInfo;
use crate::console;

#[panic_handler]
unsafe fn panic_handler(panic_info: &PanicInfo) -> ! {
    if let Some(location) = panic_info.location() {
        console::write("panic occurred in file '");
        console::write(location.file());
        console::write("'");
        // TODO use location.line() when have allocator
    } else {
        console::write("panic occurred (no location info)");
    }
    if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
        console::write(": ");
        console::write(s);
    }
    console::write("\n");
    loop {}
}
