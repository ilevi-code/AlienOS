use core::{
    arch::global_asm,
    ptr::addr_of,
    sync::atomic::{AtomicU32, Ordering},
};

use crate::{
    alloc::{Arc, Vec},
    arch::{self, PeMode},
    error::Result,
    memory_model::virt_to_phys,
    mmu::{AddressSpace, PagePerm, TranslationTable, SMALL_PAGE_SIZE},
    sched::proc::{PageTable, Process, StackPointer, State},
    spinlock::SpinLock,
};

static PROCCESSES: SpinLock<Vec<Arc<Process>>> = SpinLock::new(Vec::new());
static NEXT_PID: AtomicU32 = AtomicU32::new(1);

global_asm!(".global init_code", "init_code:", "svc #0");
extern "C" {
    static init_code: u8;
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
    // TODO replace with map_mem
    mappings.map(
        SMALL_PAGE_SIZE,
        virt_to_phys(addr_of!(init_code) as *mut u8).addr(),
        SMALL_PAGE_SIZE,
        PagePerm::UserRo,
        true,
        true,
    )?;
    init.page_table = PageTable(mappings.get_base());
    mappings.apply_user();

    let mut stack = StackPointer::from_slice(&mut init.kern_stack.0);
    let rfe_stack = stack.alloc_frame::<ReturnFromExceptionStack>()?;
    rfe_stack.cspr = PeMode::User as usize;
    rfe_stack.sp = 0; // TODO
    rfe_stack.pc = SMALL_PAGE_SIZE + (addr_of!(init_code) as usize & 0xfff);
    let switch_frame = stack.alloc_frame::<SwitchFrame>()?;
    switch_frame.regs = [0; 12];
    switch_frame.pc = crate::sched::proc::return_to_user_mode as usize;
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

fn find_runnable_proc() -> Arc<Process> {
    let guard = PROCCESSES.lock();
    loop {
        for i in 0..guard.len() {
            if let Ok(_) = guard[i].state.compare_exchange(
                State::Runnable,
                State::Running,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                return Arc::clone(&guard[i]);
            }
        }
    }
}

extern "C" {
    fn stack_switch_unchecked(other_stack: *mut u8);
}

global_asm!(
    ".global stack_switch_unchecked",
    "stack_switch_unchecked:",
    "push {{r1-r12, lr}}",
    "MRS r1, cpsr",
    "push {{r1}}",
    "mov sp, r0",
    "pop {{r1-r12}}",
    "rfe sp!",
);
