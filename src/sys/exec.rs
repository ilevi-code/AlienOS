use crate::{
    alloc::Arc,
    fs::Path,
    println,
    sched::with_current,
    sys::SyscallResult,
    {interrupts::RegSet, syscall},
};

syscall!(exec);

fn exec(regs: &mut RegSet) -> SyscallResult {
    let mut dest = [0_u8; 10];
    crate::sys::copy_from_user(&mut dest, unsafe {
        core::slice::from_raw_parts(regs.r[1] as *const u8, 10)
    })?;
    let path = Path::new(&dest);
    println!("exec: {path:?}");
    let fs = with_current(|current| Arc::clone(&current.fs)).unwrap();
    let _ = fs.path_to_inode(path);
    todo!()
}
