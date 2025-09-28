use crate::{
    alloc::Vec,
    error::Result,
    interrupts::{self, RegSet},
    println,
    spinlock::SpinLock,
    sys::Errno,
};

#[repr(C)]
pub struct Syscall {
    pub func: fn(&mut RegSet),
    pub id: usize,
}

static SYSCALLS: SpinLock<Vec<fn(&mut RegSet)>> = SpinLock::new(Vec::new());

extern "C" {
    static __syscalls_start: Syscall;
    static __syscalls_end: Syscall;
}

fn no_sys(regs: &mut RegSet) {
    regs.r[0] = Errno::NoSyscall as usize
}

pub fn init_syscalls() -> Result<()> {
    let mut syscalls = Vec::<fn(&mut RegSet)>::new();
    let start = &raw const __syscalls_start;
    let end = &raw const __syscalls_end;
    let unordered_syscalls =
        unsafe { core::slice::from_raw_parts(start, end.offset_from_unsigned(start)) };
    syscalls.resize(unordered_syscalls.len(), no_sys)?;
    for syscall in unordered_syscalls {
        if syscalls.len() <= syscall.id {
            println!("syscalls usings non linear id: {}", syscall.id);
        } else {
            syscalls[syscall.id] = syscall.func;
        }
    }
    *SYSCALLS.lock() = syscalls;

    unsafe {
        interrupts::svc_handler_pointer = svc_handler as *mut extern "C" fn(*mut RegSet);
    }

    Ok(())
}

fn svc_handler(regs: *mut RegSet) {
    let regs = unsafe { &mut *regs };
    let syscall = {
        let guard = SYSCALLS.lock();
        if guard.len() <= regs.r[0] {
            no_sys
        } else {
            guard[regs.r[0]]
        }
    };
    syscall(regs);
}
