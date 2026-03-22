use crate::{
    alloc::Arc,
    error::{Error, Result},
    fs::Ext2,
    interrupts::RegSet,
    println,
    sched::with_current,
    syscall,
};

syscall!(mount);

fn mount(regs: &mut RegSet) {
    // TOOD change syscalls to return Result, and inline mount_inner
    let _ = mount_inner(regs);
}

fn mount_inner(regs: &mut RegSet) -> Result<()> {
    let mut dest = [0_u8; 4];
    crate::sys::copy_from_user(&mut dest, unsafe {
        core::slice::from_raw_parts(regs.r[2] as *const u8, 4)
    })?;
    let disk_id = regs.r[1];
    println!(
        "mounting disk {} as {}",
        disk_id,
        str::from_utf8(&dest).unwrap_or("<bad filesystem>")
    );
    let disk = crate::sys::disk::get_disk_by_id(disk_id).ok_or(Error::NoDevice)?;
    let fs = Arc::new(Ext2::new(disk)?)?;
    with_current(|current| current.fs = fs)?;
    Ok(())
}
