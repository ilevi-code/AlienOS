use crate::error::{Error, Result};
use core::{arch::global_asm, cmp::min};

pub struct User<T>(T);

impl<T: Clone> Clone for User<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: Copy> Copy for User<T> {}

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
pub fn copy_from_user(dest: &mut [u8], src: &[User<u8>]) -> Result<()> {
    let ret = unsafe {
        copy_from_user_asm(
            dest.as_mut_ptr(),
            src.as_ptr().cast::<u8>(),
            min(dest.len(), src.len()),
        )
    };
    if ret == 0 {
        Ok(())
    } else {
        Err(Error::MemoryFault)
    }
}

global_asm!(
    ".section \".text\", \"ax\"",
    ".global copy_to_user_asm",
    "copy_to_user_asm:",
    "cmp	r2, #0",
    "bxeq	lr",
    "add	r2, r1, r2",
    "sub	r0, r0, #1",
    "sub	r1, r1, #1",
    "sub	r2, r2, #1",
    "_copy_to_user_loop:",
    "ldrb	r3, [r1, #1]!",
    "cmp	r1, r2",
    "_user_store:",
    "strb	r3, [r0, #1]!",
    "bne	_copy_to_user_loop",
    "mov    r0, #0",
    "bx	    lr",
    "_user_store_fault:",
    "mvn    r0, #0",
    "bx     lr",
    ".section \"faults\"",
    ".word _user_store, _user_store_fault"
);

extern "C" {
    fn copy_to_user_asm(dest: *mut u8, src: *const u8, len: usize) -> i32;
}

#[inline]
pub fn copy_to_user(dest: &mut [User<u8>], src: &[u8]) -> Result<()> {
    let ret = unsafe {
        copy_to_user_asm(
            dest.as_mut_ptr().cast::<u8>(),
            src.as_ptr(),
            min(dest.len(), src.len()),
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
        let src = unsafe { core::slice::from_raw_parts(0x1000 as *const User<u8>, 10) };
        assert_eq!(copy_from_user(&mut dest, src), Err(Error::MemoryFault));
    }

    #[test_case]
    fn test_copy_from_valid_address() {
        let mut dest = [0_u8; 10];
        let src = [User::<u8>(0); 10];
        assert_eq!(copy_from_user(&mut dest, &src), Ok(()));
    }

    #[test_case]
    fn test_copy_to_bad_address() {
        let src = [0_u8; 10];
        let dest = unsafe { core::slice::from_raw_parts_mut(0x1000 as *mut User<u8>, 10) };
        assert_eq!(copy_to_user(dest, &src), Err(Error::MemoryFault));
    }

    #[test_case]
    fn test_copy_to_valid_address() {
        let src = [0_u8; 10];
        let mut dest = [User::<u8>(0); 10];
        assert_eq!(copy_to_user(&mut dest, &src), Ok(()));
    }
}
