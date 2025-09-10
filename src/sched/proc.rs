use crate::{
    alloc::{Box, Vec},
    mmu::SMALL_PAGE_SIZE,
    spinlock::SpinLock,
};

enum Errno {
    FAULT,
    INVAL,
}

struct User<T: ?Sized>(T);

trait File {
    fn read(&mut self, buf: User<[u8]>) -> Result<(), Errno>;
}

use core::{arch::global_asm, ops::Deref, sync::atomic::AtomicUsize};

struct Arc<T> {
    ptr: *const T,
    ref_count: *const AtomicUsize,
}

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

enum State {
    Running,
    Zombie,
}

struct PageTable(usize);

struct Process {
    pid: u32,
    regs: [usize; 16],
    page_table: PageTable,
    kern_stack: Box<[u8; SMALL_PAGE_SIZE]>,
    fd: Vec<Option<SpinLock<Box<dyn File>>>>,
    state: State,
}

extern "C" {
    fn stack_switch_unchecked(other_stack: *mut u8);
}

global_asm!(
    ".global return_to_user_mode",
    "return_to_user_mode:",
    "ldmfd sp!, {{r0-r12}}",
    "rfeia sp!",
    "movs pc, lr"
);

global_asm!(
    ".global stack_switch",
    "stack_switch:",
    "stm sp!, {{r1-r15}}",
    "MSR cpsr, r0",
    "push r0",
    "mov sp, r0",
    "pop r0",
    "mrs cpsr, r0",
    "ldmfd sp!, {{r1-r15}}",
);
