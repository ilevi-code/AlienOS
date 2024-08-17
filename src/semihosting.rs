use core::arch::asm;

const SYS_EXIT_EXTENDED: u32 = 0x20;

const ADP_Stopped_ApplicationExit: u32 = 0x20026;

pub(crate) fn shutdown(exit_code: u8) -> ! {
    unsafe {
        // Semihosting call.
        let ret = [ADP_Stopped_ApplicationExit, exit_code as u32];
        asm!(
            "SVC #0x00123456",
            in("r0") SYS_EXIT_EXTENDED,
            in("r1") ret.as_ptr()
        );
    }
    // In case something failed - hang.
    loop {}
}
