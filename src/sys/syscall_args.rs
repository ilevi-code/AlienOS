use core::convert::From;
use core::slice;

use crate::alloc::{Arc, Box, Vec};
use crate::error::{Result, Error};
use crate::fs::File;
use crate::sched::{FileTableEntry, with_current};
use crate::spinlock::SpinLockGuard;
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
        let user_bytes = self.get_user_bytes()?;

        let mut path = Vec::new();
        path.resize(user_bytes.len(), 0)?;

        copy_from_user(&mut path[..], user_bytes)?;

        Ok(path)
    }

    pub fn get_user_bytes(&mut self) -> Result<&'static [User<u8>]> {
        let addr = self.next_reg()? as *const User<u8>;
        if addr.is_null() || !addr.is_aligned() {
            return Err(Error::MemoryFault);
        }
        let len = self.next_reg()?;
        if len > PATH_MAX {
            return Err(Error::NameTooLong);
        }
        Ok(
        unsafe { slice::from_raw_parts(addr, len) })
    }

    pub fn get_fd(&mut self) -> Result<FileTableEntry> {
        let fd = self.next_reg()?;
        let file = with_current(|current| -> Option<FileTableEntry> {
            let fd_table = current.fds.lock();
            let file_lock = fd_table.get(fd)?.as_ref()?;
            Some(Arc::clone(file_lock))
        })?;
        file.ok_or(Error::BadFd)
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
