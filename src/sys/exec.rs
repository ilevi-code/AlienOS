use core::fmt::Debug;

use crate::alloc::{Box, Vec};
use crate::fs::Inode;
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

use crate::error::Result;

fn path_to_inode(path: &Path) -> Result<Box<Inode>> {
    let mut aux_buf = Vec::<u8>::new();
    aux_buf.resize(1024, 0)?;
    todo!();
}
