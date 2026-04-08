#![no_std]
#![no_main]

const SYS_EXIT: usize = 2;
const SYS_OPEN: usize = 3;

use core::{arch::asm, panic::PanicInfo, result::Result};

#[panic_handler]
fn panic_handler(_painc_info: &PanicInfo) -> ! {
    exit(1)
}

fn exit(exit_code: u32) -> ! {
    unsafe { asm!("svc #0", in("r0") SYS_EXIT, in("r1") exit_code) }
    unreachable!()
}

enum Errno {
    Unknown,
}

fn open(path: &str) -> Result<usize, Errno> {
    let result: i32;
    let fd;
    unsafe {
        asm!(
            "svc #0",
            "mov {result}, r0",
            "mov {fd}, r1",
            result = out(reg) result,
            fd = out(reg) fd,
            in("r0") SYS_OPEN,
            in("r1") path.as_ptr(),
            in("r2") path.len()
        )
    }
    if result == 0 {
        Ok(fd)
    } else {
        Err(Errno::Unknown)
    }
}

#[unsafe(no_mangle)]
extern "C" fn _init() {
    let exit_code = match main() {
        Ok(_) => 0,
        Err(_) => 1,
    };
    exit(exit_code)
}

fn main() -> Result<(), Errno> {
    let _console = open("/dev/console")?;
    Ok(())
}
