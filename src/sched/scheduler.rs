use core::{
    arch::global_asm,
    ptr::{addr_of_mut, null_mut, NonNull},
    slice,
    sync::atomic::{AtomicU32, Ordering},
};

use crate::{
    alloc::{Arc, Vec},
    arch::{self, PeMode},
    error::{Error, Result},
    heap,
    interrupts::{irq_state_save, without_irq},
    mmu::{Page, PagePerm},
    per_cpu,
    phys::Phys,
    sched::proc::{Process, StackPointer, State},
    spinlock::SpinLock,
};

static PROCCESSES: SpinLock<Vec<Arc<Process>>> = SpinLock::new(Vec::new());
static NEXT_PID: AtomicU32 = AtomicU32::new(1);

per_cpu!(CURRENT: Option<NonNull<Process>> = None);
per_cpu!(SCHED_STACK: *mut u8 = null_mut());

global_asm!(
    ".section \".text\", \"ax\"",
    ".type init_code_start, \"function\"",
    ".global init_code_start",
    "init_code_start:",
    // push "ext2" to stack
    "movw r0, #0x7865",
    "movt r0, #0x3274",
    "mov r1, #0",
    "push {{r0, r1}}",
    "mov r2, sp",
    "mov r3, #4",
    // disk #0
    "mov r1, 0",
    // sys_mount
    "mov r0, 1",
    // mount(0, "ext2")
    "svc #0",
    // Push "/sbin/init" to stack
    "movw r0, #0x732f",
    "movt r0, #0x6962",
    "movw r1, #0x2f6e",
    "movt r1, #0x6e69",
    "movw r2, #0x7469",
    "movt r2, #0",
    "push {{r0, r1, r2}}",
    "mov r1, sp",
    "mov r2, #10",
    // push null
    "mov r0, 0",
    "push {{r0}}",
    "mov r3, sp",
    // exec("/sbin/init", null)
    "svc #0",
    "mov r1, 1",
    "mov r0, 2",
    // exit(1)
    "svc #0",
    ".global init_code_end",
    "init_code_end:",
);

extern "C" {
    static init_code_start: u8;
    static init_code_end: u8;
}

fn get_init_code() -> &'static [u8] {
    unsafe {
        slice::from_raw_parts(
            &raw const init_code_start,
            (&raw const init_code_end).offset_from_unsigned(&raw const init_code_start),
        )
    }
}

fn clone_init_code() -> Result<*const Page> {
    let code_page = heap::alloc::<Page>()?;
    let init_code = get_init_code();
    debug_assert!(init_code.len() < size_of::<Page>());
    // Safety:
    // All bytes until `length` are uniquely owned
    unsafe { slice::from_raw_parts_mut(code_page.cast::<u8>(), init_code.len()) }
        .copy_from_slice(init_code);
    Ok(code_page)
}

#[repr(C)]
struct ReturnFromExceptionStack {
    sp: usize,
    pc: usize,
    cspr: usize,
}

#[repr(C)]
struct SwitchFrame {
    regs: [usize; 11],
    pc: usize,
    cspr: usize,
}

pub fn setup_init_proc() -> Result<()> {
    let pid = NEXT_PID.fetch_add(1, Ordering::Relaxed);
    let mut init = Process::with_pid(pid)?;

    let code_page = clone_init_code()?;
    let mapped_code = init.page_table.map_memory(
        Phys::from_virt(Page::as_slice_ptr(code_page)),
        PagePerm::UserRo,
    )?;

    setup_proc_stack(&mut init, mapped_code.as_ptr().addr())?;
    let init = Arc::new(init)?;

    PROCCESSES.lock().push(init)?;
    Ok(())
}

pub fn setup_proc_stack(proc: &mut Process, entrypoint: usize) -> Result<()> {
    let stack = heap::alloc::<Page>()?;
    let mapped_stack = proc
        .page_table
        .map_memory(Phys::from_virt(Page::as_slice_ptr(stack)), PagePerm::UserRw)?;

    let mut stack = StackPointer::from_slice(&mut proc.kern_stack.0);
    let rfe_stack = stack.alloc_frame::<ReturnFromExceptionStack>()?;
    rfe_stack.cspr = PeMode::User as usize;
    rfe_stack.sp = mapped_stack.as_ptr().addr() + size_of::<Page>();
    rfe_stack.pc = entrypoint;
    let switch_frame = stack.alloc_frame::<SwitchFrame>()?;
    switch_frame.regs = [0; 11];
    switch_frame.pc = return_to_user_mode as *const () as usize;
    switch_frame.cspr = arch::get_cpsr();

    proc.sp = stack.into_sp();
    Ok(())
}

pub fn sched() -> ! {
    loop {
        let proc = find_runnable_proc();
        CURRENT.replace(Some(NonNull::from_ref(&*proc)));
        let sched_stack = SCHED_STACK.as_ptr();
        proc.page_table.apply_user();
        without_irq(|| {
            irq_state_save(|| unsafe {
                stack_switch_unchecked(sched_stack, proc.sp);
            })
        });
        // When stack_switch returns, it means the process yielded, and changed it's state from
        // Running to something else.
    }
}

pub fn yield_to_sched(old_sp: *mut *mut u8) {
    let sched_stack = SCHED_STACK.get();
    // The scheduler switched with interrupts disabled, so we must return to it with interrupts
    // still disabled.
    without_irq(|| irq_state_save(|| unsafe { stack_switch_unchecked(old_sp, sched_stack) }))
}

pub fn sleep_on(chan: usize) -> Result<()> {
    let old_sp = with_current(|current| {
        current.state.store(State::Sleeping, Ordering::Relaxed);
        current.chan.store(chan, Ordering::Relaxed);
        addr_of_mut!(current.sp)
    })?;
    yield_to_sched(old_sp);
    Ok(())
}

pub fn with_current<Ret, F: FnOnce(&mut Process) -> Ret>(f: F) -> Result<Ret> {
    let mut current = CURRENT.get().ok_or(Error::NoCurrentProcess)?;
    Ok(f(unsafe { current.as_mut() }))
}

pub fn wakeup(chan: usize) {
    without_irq(|| wakeup_irq_disabled(chan))
}

fn wakeup_irq_disabled(chan: usize) {
    let guard = PROCCESSES.lock();
    for proccess in &*guard {
        if proccess.chan.load(Ordering::Acquire) == chan {
            let _ = proccess.state.compare_exchange(
                State::Sleeping,
                State::Runnable,
                Ordering::Acquire,
                Ordering::Relaxed,
            );
        }
    }
}

fn find_runnable_proc() -> Arc<Process> {
    loop {
        if let Some(proc) = without_irq(search_runnable_proc) {
            return proc;
        }
    }
}

fn search_runnable_proc() -> Option<Arc<Process>> {
    let guard = PROCCESSES.lock();
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
            return Some(Arc::clone(&guard[i]));
        }
    }
    None
}

extern "C" {
    fn stack_switch_unchecked(old_stack: *mut *mut u8, other_stack: *mut u8);
    fn return_to_user_mode(other_stack: *mut u8);
}

global_asm!(
    ".section \".text\", \"ax\"",
    ".global stack_switch_unchecked",
    "stack_switch_unchecked:",
    "sub sp, #4",
    "push {{lr}}",
    "push {{r2-r12}}",
    "add sp, #52",
    "MRS r2, cpsr",
    "push {{r2}}",
    "sub sp, #48",
    "str sp, [r0]",
    "mov sp, r1",
    "pop {{r2-r12}}",
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
