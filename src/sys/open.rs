use core::{cmp::min, slice};

use crate::{
    alloc::Arc, fs::{FileSystem, Path}, interrupts::RegSet, sched::with_current, spinlock::SpinLock, sys::{SyscallResult, User}, syscall
};

syscall!(open);

fn open(regs: &mut RegSet) -> SyscallResult {
    let mut dest = [0_u8; 20];
    crate::sys::copy_from_user(&mut dest, unsafe {
        slice::from_raw_parts(regs.r[1] as *const User<u8>, regs.r[2])
    })?;

    let path = Path::new(&dest[..min(dest.len(), regs.r[2])]);
    let fs = with_current(|current| Arc::clone(&current.fs))?;
    let file = FileSystem::open(Arc::clone(&fs), path)?;
    let fd = with_current(|current| -> crate::error::Result<usize> {
        let mut fds = current.fds.lock();
        // TODO check for empty spaces
        fds.push(Some(SpinLock::new(file)))?;
        Ok(fds.len())
    })??;
    Ok(fd)
}
