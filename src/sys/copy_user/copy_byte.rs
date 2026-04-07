use core::arch::global_asm;

use crate::{
    error::{Error, Result},
    sys::User,
};

global_asm!(
    ".section \".text\", \"ax\"",
    ".global copy_byte_to_user_asm",
    "copy_byte_to_user_asm:",
    "_user_store_byte:",
    "strb	r1, [r0]",
    "mov    r0, #0",
    "bx	    lr",
    "_user_store_byte_fault:",
    "mvn    r0, #0",
    "bx     lr",
    ".section \"faults\"",
    ".word _user_store_byte, _user_store_byte_fault"
);

extern "C" {
    fn copy_byte_to_user_asm(dest: *mut u8, val: u8) -> i32;
}

#[inline]
pub fn copy_byte_to_user(dest: &mut User<u8>, val: u8) -> Result<()> {
    let ret = unsafe { copy_byte_to_user_asm(core::ptr::from_mut(dest).cast::<u8>(), val) };
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
    fn test_copy_byte_to_valid_address() {
        let mut dest = User::<u8>(0);
        assert_eq!(copy_byte_to_user(&mut dest, 1), Ok(()));
        assert_eq!(dest.0, 1);
    }

    #[test_case]
    fn test_copy_byte_to_bad_address() {
        let dest = unsafe { (0x1000 as *mut User<u8>).as_mut() }.unwrap();
        assert_eq!(copy_byte_to_user(dest, 1), Err(Error::MemoryFault));
    }
}
