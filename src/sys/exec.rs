use core::fmt::Debug;

use crate::{interrupts::RegSet, syscall};

syscall!(exec);

struct Path {
    bytes: [u8],
}

impl Path {
    fn new(bytes: &[u8]) -> &Self {
        unsafe { &*(bytes as *const [u8] as *const Path) }
    }
}

impl Debug for Path {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(unsafe { str::from_utf8_unchecked(&self.bytes) })
    }
}

fn exec(regs: &mut RegSet) {
    let mut dest = [0_u8; 10];
    let res = crate::sys::copy_from_user(&mut dest, unsafe {
        core::slice::from_raw_parts(regs.r[1] as *const u8, 10)
    });
    use crate::println;
    match res {
        Ok(_) => {
            let path = Path::new(&dest);
            println!("exec: {path:?}");
        }
        Err(e) => println!("{:?}", e),
    };
    crate::semihosting::shutdown(0);
}
