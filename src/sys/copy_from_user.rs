use crate::error::{Error, Result};
use core::{arch::global_asm, cmp::min};

pub struct User<T>(T);

impl User<&[u8]> {
    pub fn from_raw_parts(user_ptr: *const u8, len: usize) -> Self {
        Self(unsafe { core::slice::from_raw_parts(user_ptr, len) })
    }
}

global_asm!(
    ".section \".text\", \"ax\"",
    ".global copy_from_user_asm",
    "copy_from_user_asm:",
    "cmp	r2, #0",
    "bxeq	lr",
    "add	r2, r1, r2",
    "sub	r0, r0, #1",
    "sub	r1, r1, #1",
    "sub	r2, r2, #1",
    "_copy_from_user_loop:",
    "_user_load:",
    "ldrb	r3, [r1, #1]!",
    "cmp	r1, r2",
    "strb	r3, [r0, #1]!",
    "bne	_copy_from_user_loop",
    "mov    r0, #0",
    "bx	    lr",
    "_user_fault:",
    "mvn    r0, #0",
    "bx     lr",
    ".section \"faults\"",
    ".word _user_load, _user_fault"
);

extern "C" {
    fn copy_from_user_asm(dest: *mut u8, src: *const u8, len: usize) -> i32;
}

#[inline]
pub fn copy_from_user(dest: &mut [u8], src: User<&[u8]>) -> Result<()> {
    let ret = unsafe {
        copy_from_user_asm(
            dest.as_mut_ptr(),
            src.0.as_ptr(),
            min(dest.len(), src.0.len()),
        )
    };
    if ret == 0 {
        Ok(())
    } else {
        Err(Error::MemoryFault)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_copy_from_bad_address() {
        let mut dest = [0_u8; 10];
        let src = unsafe { core::slice::from_raw_parts(0x1000 as *const u8, 10) };
        assert_eq!(copy_from_user(&mut dest, src), Err(Error::MemoryFault));
    }

    #[test_case]
    fn test_copy_from_valid_address() {
        let mut dest = [0_u8; 10];
        let src = [0_u8; 10];
        assert_eq!(copy_from_user(&mut dest, &src), Ok(()));
    }
}
