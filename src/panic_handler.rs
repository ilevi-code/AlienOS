use core::panic::PanicInfo;

use crate::console;

mod semihosting {
    use core::arch::asm;

    pub fn shutdown() -> ! {
        unsafe {
            // Semihosting call.
            // 0x18 is angel_SWIreason_ReportException
            // 0x20026 is ADP_Stopped_ApplicationExit
            asm!(
                "MOV r0, #0x18",
                "MOVT r1, #2",
                "MOV r1, #0x26",
                "SVC #0x00123456",
            );
        }
        // In case something failed - hang.
        loop {}
    }
}

#[panic_handler]
unsafe fn panic_handler(panic_info: &PanicInfo) -> ! {
    let res = console::write_args(format_args!("{}\n", panic_info));
    if res.is_err() {
        console::write("\n--- error formatting panic info ---\n");
    }
    semihosting::shutdown()
}
