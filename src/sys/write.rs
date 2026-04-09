use crate::{
    interrupts::RegSet, sys::{SyscallArgs, SyscallResult}, syscall
};

syscall!(write);

fn write(regs: &mut RegSet) -> SyscallResult {
    let mut args = Into::<SyscallArgs>::into(&regs.r[..]);
    let file = args.get_fd()?;
    let data = args.get_user_bytes()?;
    let mut file = file.lock();
    file.write(data)?;
    Ok(data.len())
}
