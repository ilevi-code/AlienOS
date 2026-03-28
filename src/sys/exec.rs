use crate::{
    alloc::{Arc, Box},
    error::Result,
    fs::{File, FileSystem, Path},
    interrupts::RegSet,
    println,
    sched::with_current,
    sys::{SyscallResult, User},
    syscall,
};

syscall!(exec);

fn exec(regs: &mut RegSet) -> SyscallResult {
    let mut dest = [0_u8; 10];
    crate::sys::copy_from_user(
        &mut dest,
        User::<&[u8]>::from_raw_parts(regs.r[1] as *const u8, 10),
    )?;
    let path = Path::new(&dest);
    println!("exec: {path:?}");
    let fs = with_current(|current| Arc::clone(&current.fs))?;
    let file = FileSystem::open(Arc::clone(&fs), path)?;
    exec_load(file)?;
    Ok(0)
}

fn exec_load(elf: Box<dyn File>) -> Result<()> {
    todo!();
}
