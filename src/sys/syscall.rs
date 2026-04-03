use crate::{
    alloc::Vec,
    error::Result,
    interrupts::{self, RegSet},
    println,
    spinlock::SpinLock,
    sys::{Errno, SyscallResult},
};

type SyscallFn = extern "Rust" fn(&mut RegSet) -> SyscallResult;

#[repr(C)]
pub struct Syscall {
    pub func: SyscallFn,
    pub id: usize,
}

pub enum SyscallNumber {
    Exec = 0,
    Mount = 1,
}

#[macro_export]
macro_rules! syscall {
    ($name:ident) => {
        use paste::paste;
        paste! {
            #[link_section = "syscalls"]
            #[used]
            static [< _SYSCALL_ $name:upper >]: $crate::sys::Syscall = $crate::sys::Syscall {
                func: $name,
                id: $crate::sys::SyscallNumber:: [< $name:camel >] as usize
            };
        }
    };
}

static SYSCALLS: SpinLock<Vec<SyscallFn>> = SpinLock::new(Vec::new());

extern "Rust" {
    static __syscalls_start: Syscall;
    static __syscalls_end: Syscall;
}

fn no_sys(_regs: &mut RegSet) -> SyscallResult {
    Err(Errno::NoSyscall)
}

pub fn init_syscalls() -> Result<()> {
    let mut syscalls = Vec::<SyscallFn>::new();
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
    regs.lr += 4;
    regs.r[0] = match syscall(regs) {
        Ok(return_value) => return_value,
        Err(error) => error as usize,
    };
}
