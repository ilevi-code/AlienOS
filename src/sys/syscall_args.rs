use core::convert::From;
use core::slice;

use crate::alloc::Vec;
use crate::error::{Result, Error};
use crate::sys::{User, copy_from_user};

const PATH_MAX: usize = 1024;

pub struct SyscallArgs<'a> {
    reg_index: usize,
    regs: &'a [usize]
}

impl<'a> From<&'a [usize]> for SyscallArgs<'a> {
    fn from(value: &'a [usize]) -> Self {
        SyscallArgs{regs: value, reg_index: 1 }
    }
}

impl<'a> SyscallArgs<'a> {
    pub fn get_string(&mut self) -> Result<Vec<u8>> {
        let mut path = Vec::new();

        let addr = self.next_reg()? as *const User<u8>;
        if addr.is_null() || !addr.is_aligned() {
            return Err(Error::MemoryFault);
        }
        let len = self.next_reg()?;
        if len > PATH_MAX {
            return Err(Error::NameTooLong);
        }

        path.resize(len, 0)?;
        copy_from_user(&mut path[..], unsafe { slice::from_raw_parts(addr, len) })?;

        Ok(path)
    }

    pub fn next_reg(&mut self) -> Result<usize> {
        match self.regs.get(self.reg_index) {
            Some(reg) => {
                self.reg_index += 1;
                Ok(*reg)
            }
            None => Err(Error::EndOfSyscallArgs),
        }
    }

    pub fn next_enum<T: From<u32>>(&mut self) -> Result<usize> {
        let value = self.next_reg()?;
        Ok(value.into())
    }
}
