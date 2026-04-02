#![no_std]
#![no_main]

const SYS_EXIT: usize = 2;

use core::{arch::asm, panic::PanicInfo};

#[panic_handler]
fn panic_handler(_painc_info: &PanicInfo) -> ! {
    exit(1)
}

fn exit(exit_code: u32) -> ! {
    unsafe { asm!("svc #0", in("r0") SYS_EXIT, in("r1") exit_code) }
    unreachable!()
}

#[unsafe(no_mangle)]
extern "C" fn _init() {
    exit(0)
}
