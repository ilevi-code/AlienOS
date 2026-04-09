use crate::log;

use crate::{
    alloc::Arc,
    error::Error,
    fs::Ext2,
    interrupts::RegSet,
    sched::with_current,
    sys::{SyscallArgs, SyscallResult},
    syscall,
};

syscall!(mount);

fn mount(regs: &mut RegSet) -> SyscallResult {
    let mut args = Into::<SyscallArgs>::into(&regs.r[..]);
    let disk_id = args.next_reg()?;
    let fs_type = args.get_string()?;

    log!(
        "mounting disk {} as {}",
        disk_id,
        str::from_utf8(&fs_type[..]).unwrap_or("<bad filesystem>")
    );

    let disk = crate::sys::disk::get_disk_by_id(disk_id).ok_or(Error::NoDevice)?;
    // TODO actually check the requested filesystem
    let fs = Arc::new(Ext2::new(disk)?)?;
    with_current(|current| current.fs = fs)?;
    Ok(0)
}
