use core::{
    arch::global_asm,
    sync::atomic::{AtomicU32, Ordering},
};

use crate::{
    alloc::{Arc, Vec},
    arch::{self, PeMode},
    error::Result,
    heap,
    mmu::{AddressSpace, Page, PagePerm, TranslationTable},
    phys::Phys,
    sched::proc::{PageTable, Process, StackPointer, State},
    spinlock::SpinLock,
};

static PROCCESSES: SpinLock<Vec<Arc<Process>>> = SpinLock::new(Vec::new());
static NEXT_PID: AtomicU32 = AtomicU32::new(1);

global_asm!(
    ".section \".text\", \"ax\"",
    ".type init_code, \"function\"",
    ".global init_code",
    "init_code:",
    // Push "/sbin/init" to stack
    "movw r0, #0x732f",
    "movt r0, #0x6962",
    "movw r1, #0x2f6e",
    "movt r1, #0x6e69",
    "movw r2, #0x7469",
    "movt r2, #0",
    "push {{r0, r1, r2}}",
    "mov r1, sp",
    "mov r0, 0",
    "push {{r0}}",
    "mov r2, sp",
    "svc #0",
    ".global init_code_end",
    "init_code_end:",
);

extern "C" {
    static init_code: u8;
    static init_code_end: u8;
}

fn get_init_code() -> *const [u8] {
    unsafe {
        core::ptr::slice_from_raw_parts(
            &raw const init_code,
            (&raw const init_code_end).offset_from_unsigned(&raw const init_code),
        )
    }
}

#[repr(C)]
struct ReturnFromExceptionStack {
    sp: usize,
    pc: usize,
    cspr: usize,
}

#[repr(C)]
struct SwitchFrame {
    regs: [usize; 12],
    pc: usize,
    cspr: usize,
}

pub fn setup_init_proc() -> Result<()> {
    let pid = NEXT_PID.fetch_add(1, Ordering::Relaxed);
    let mut init = Process::with_pid(pid)?;

    let mut mappings = TranslationTable::new(AddressSpace::User)?;
    let mapped_code = mappings.map_memory(Phys::from_virt(get_init_code()), PagePerm::UserRo)?;
    init.page_table = PageTable(mappings.get_base());
    mappings.apply_user();

    let stack = heap::alloc::<Page>()?;
    let mapped_stack =
        mappings.map_memory(Phys::from_virt(Page::as_slice_ptr(stack)), PagePerm::UserRo)?;

    let mut stack = StackPointer::from_slice(&mut init.kern_stack.0);
    let rfe_stack = stack.alloc_frame::<ReturnFromExceptionStack>()?;
    rfe_stack.cspr = PeMode::User as usize;
    rfe_stack.sp = mapped_stack.as_ptr().addr() + size_of::<Page>();
    rfe_stack.pc = mapped_code.as_ptr().addr();
    let switch_frame = stack.alloc_frame::<SwitchFrame>()?;
    switch_frame.regs = [0; 12];
    switch_frame.pc = return_to_user_mode as *const () as usize;
    switch_frame.cspr = arch::get_cpsr();

    init.sp = stack.into_sp();
    let init = Arc::new(init)?;

    PROCCESSES.lock().push(init)?;
    Ok(())
}

pub fn sched() -> ! {
    loop {
        let proc = find_runnable_proc();
        unsafe {
            stack_switch_unchecked(proc.sp);
        }
    }
}

pub fn wakeup(chan: usize) {
    let guard = PROCCESSES.lock();
    for i in 0..guard.len() {
        if guard[i].chan.load(Ordering::Acquire) == chan {
            let _ = guard[i].state.compare_exchange(
                State::Sleeping,
                State::Runnable,
                Ordering::Acquire,
                Ordering::Relaxed,
            );
        }
    }
}

fn find_runnable_proc() -> Arc<Process> {
    let guard = PROCCESSES.lock();
    loop {
        for i in 0..guard.len() {
            if guard[i]
                .state
                .compare_exchange(
                    State::Runnable,
                    State::Running,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                return Arc::clone(&guard[i]);
            }
        }
    }
}

extern "C" {
    fn stack_switch_unchecked(other_stack: *mut u8);
    fn return_to_user_mode(other_stack: *mut u8);
}

global_asm!(
    ".section \".text\", \"ax\"",
    ".global stack_switch_unchecked",
    "stack_switch_unchecked:",
    "push {{r1-r12, lr}}",
    "MRS r1, cpsr",
    "push {{r1}}",
    "mov sp, r0",
    "pop {{r1-r12}}",
    "rfe sp!",
);

global_asm!(
    ".section \".text\", \"ax\"",
    ".global return_to_user_mode",
    "return_to_user_mode:",
    "ldm sp, {{sp}}^",
    "add sp, sp, 4",
    "rfe sp!",
);
