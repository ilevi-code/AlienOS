use crate::{interrupts::RegSet, sched::with_current, sys::SyscallResult, syscall};

syscall!(exit);

fn exit(regs: &mut RegSet) -> SyscallResult {
    with_current(|current| {
        if current.pid == 1 {
            panic!("Init exited! (code={})", regs.r[1]);
        }
    })?;
    todo!();
}
