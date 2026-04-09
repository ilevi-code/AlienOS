use crate::{
    alloc::Arc,
    fs::{FileSystem, Path},
    interrupts::RegSet,
    sched::with_current,
    spinlock::SpinLock,
    sys::{SyscallArgs, SyscallResult},
    syscall,
};

syscall!(open);

fn open(regs: &mut RegSet) -> SyscallResult {
    let mut args = Into::<SyscallArgs>::into(&regs.r[..]);
    let path_buf = args.get_string()?;

    let path = Path::new(&path_buf[..]);
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
