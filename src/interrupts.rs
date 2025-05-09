use core::ptr::addr_of;

#[repr(C)]
#[derive(Default, Clone)]
struct RegSet {
    r: [usize; 13],
    lr: usize,
    cpsr: usize,
}

core::arch::global_asm!(
    ".section interrupt_table, \"ax\"",
    ".global interrupt_table_start",
    "interrupt_table_start:",
    "",
    "nop",                   // reset handler
    "nop",                   // undefined instruction handler
    "b _svc_handler",        // svc handler
    "nop",                   // prefetch abort
    "b _data_abort_handler", // data abort
    "nop",                   // unused
    "b _irq_handler",        // IRQ
    "nop",                   // FIQ
    "",
    "",
    "_data_abort_handler:",
    "sub lr, lr, #8", // The lr registers will point to 8 bytes after the faulting instruction
    "srsdb #0x17!",   // push LR_abt and CPSR_abt to the stack.
    "push {{r0-r12}}",
    "mov r0, sp",
    "ldr r1, data_abort_handler_pointer",
    "blx r1",
    "pop {{r0-r12}}",
    "rfeia sp!", // load LR and SPSR from the stack
    ".global data_abort_handler_pointer",
    "data_abort_handler_pointer:",
    ".word 0x0",
    "",
    "_irq_handler:",
    "sub lr, lr, #4", // The lr registers will point to 4 bytes after the faulting instruction
    "srsdb #0x12!",   // push LR_irq and CPSR_irq to the stack.
    "push {{r0-r12}}",
    "mov r0, sp",
    "ldr r1, irq_handler_pointer",
    "blx r1",
    "pop {{r0-r12}}",
    "rfeia sp!", // load LR and SPSR from the stack
    ".global irq_handler_pointer",
    "irq_handler_pointer:",
    ".word 0x0",
    "",
    "_svc_handler:",
    // check this
    "sub lr, lr, #4", // The lr registers will point to 4 bytes after the faulting instruction
    "srsdb #0x13!",   // push LR_svc and CPSR_svc to the stack.
    "push {{r0-r12}}",
    "mov r0, sp",
    "ldr r1, svc_handler_pointer",
    "blx r1",
    "pop {{r0-r12}}",
    "rfeia sp!", // load LR and SPSR from the stack
    ".global svc_handler_pointer",
    "svc_handler_pointer:",
    ".word 0x0",
);
extern "C" {
    static interrupt_table_start: u32;
    static mut data_abort_handler_pointer: *mut extern "C" fn(*mut RegSet);
    static mut irq_handler_pointer: *mut extern "C" fn(*mut RegSet);
    static mut svc_handler_pointer: *mut extern "C" fn(*mut RegSet);
}

fn read_fault_register() -> usize {
    let fault_address: usize;
    unsafe {
        core::arch::asm!("MRC p15, 0, {}, c6, c0, 0", out(reg) fault_address);
    }
    fault_address
}

#[no_mangle]
extern "C" fn data_abort_handler(reg_set: *mut RegSet) {
    crate::console::println!(
        "fault acessing address 0x{:x} from 0x{:x}",
        read_fault_register(),
        unsafe { &*reg_set }.lr,
    );
}

fn set_high_exception_vector_address(address: usize) {
    unsafe {
        core::arch::asm!("MCR p15, 0, {}, c12, c0, 0", in(reg) address);
    }
}

fn set_data_abort_handler(handler: extern "C" fn(*mut RegSet)) {
    unsafe {
        data_abort_handler_pointer = handler as *mut extern "C" fn(*mut RegSet);
    }
}

// extern crate alloc;
// struct ExceptionHandlerStacks {
//     data_abort_stack: alloc::boxed::Box<[u8]>,
// }

fn set_data_abort_stack(stack: usize) {
    unsafe {
        core::arch::asm!(
            "msr CPSR_c, #0x17",
            "mov sp, {}",
            "msr CPSR_c, #0x13",
            in(reg) stack,
        );
    }
}

mod timer {
    pub(crate) struct VirtualCounter;

    impl VirtualCounter {
        pub(crate) fn enable(&mut self) {
            // SAFETY: no memory changes, just enabling timter intrrupts.
            unsafe {
                // set the enable bit in CNTV_CTL
                core::arch::asm!(
                    "MCR p15, 0, {tmp}, c14, c3, 1",
                    tmp = in(reg) 1,
                );
            }
        }

        pub(crate) fn arm(&mut self, ticks: usize) {
            // SAFETY: no memory changes, just moving to a tick-counting register.
            unsafe {
                // arm CNTV_TVAL
                core::arch::asm!("MCR p15, 0, {}, c14, c3, 0", in(reg) ticks);
            }
        }

        /// Returns how many clock ticks there are in a second.
        pub(crate) fn frequency(&self) -> usize {
            let tick_frequency: usize;
            // SAFETY: reading from a register.
            unsafe {
                // Read from CNTFRQ
                core::arch::asm!("MRC p15, 0, {}, c14, c0, 0", out(reg) tick_frequency);
            }
            tick_frequency
        }

        pub(crate) fn irq_id(&self) -> usize {
            // Documented in the ARM docs, under "The processor timers", "Interrupts" subsection.
            27
        }
    }
}

extern "C" fn irq_handler(reg_set: *mut RegSet) {
    let gicc = super::gic::get_gicc();
    let int_num = gicc.current_interrupt_number();
    crate::console::println!("irq number #{}!\n", int_num);
    let mut timer = timer::VirtualCounter;
    timer.arm(timer.frequency());
    gicc.signal_end(int_num);
}

pub(crate) fn init_interrupt_handler() {
    set_data_abort_handler(data_abort_handler);
    // TODO setup stack for:
    // abort (mode 0b10111)
    // FIQ (mode 0b10001)
    // IRQ (mode 0b10010)
    // currently, SP_abrt is 0, so the stack grow down from 0xffff_ffff.
    set_high_exception_vector_address(addr_of!(interrupt_table_start) as usize);

    unsafe {
        svc_handler_pointer = svc_handler as *mut extern "C" fn(*mut RegSet);
    }

    unsafe {
        irq_handler_pointer = irq_handler as *mut extern "C" fn(*mut RegSet);
    }
    unsafe {
        let gicc = super::gic::get_gicc();
        gicc.enable_singaling_to_cpu();
        gicc.set_prio_mask(super::gic::Gicc::ALLOW_ALL);

        // let gicd = &mut *super::gic::GICD.load(core::sync::atomic::Ordering::Acquire);
        // // enable forwarding interrupts
        // gicd.ctlr = 1;

        // // GICD_ISENABLER: IRQ 27 -> ISENABLER1 (IRQ 32..63)
        // gicd.isenabler[0] = 1 << 27;
        let gicd = super::gic::get_gicd();
        gicd.enable_forarding();
        gicd.enable_interrupt(27);
    }

    unsafe {
        core::arch::asm!("CPSIE i");
        let mut timer = timer::VirtualCounter;
        timer.enable();
        timer.arm(timer.frequency());
    }
}

pub(crate) fn svc_handler(reg_set: *mut RegSet) {
    crate::console::println!("syscall!");
    crate::semihosting::shutdown(0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spinlock::SpinLock;

    #[derive(Clone)]
    struct AbortInfo {
        regs: RegSet,
        addr: usize,
    }

    static ABORT_INFO: SpinLock<AbortInfo> = SpinLock::new(AbortInfo {
        regs: RegSet {
            r: [0; 13],
            cpsr: 0,
            lr: 0,
        },
        addr: 0,
    });

    extern "C" fn dummy_data_abort_handler(reg_set: *mut RegSet) {
        let reg_set = unsafe { &mut *reg_set };
        let mut abort_info = ABORT_INFO.lock();
        abort_info.regs = reg_set.clone();
        abort_info.addr = read_fault_register();
        reg_set.lr += 4; // adjust lr to skip the faulting instruction
    }

    #[test_case]
    fn test_data_abort_reported_fault_address() {
        init_interrupt_handler();
        set_data_abort_handler(dummy_data_abort_handler);

        let addr: usize = 0xaeadbeef;
        let pc: usize;

        unsafe {
            core::arch::asm!(
                // pc is loaded 8 bytes ahead of current instruction
                "sub {},pc,#4",
                "str r1,[{}]",
                out(reg) pc,
                in(reg) addr
            );
        }

        let abort_info = ABORT_INFO.lock();
        assert_eq!(abort_info.regs.lr, pc);
        assert_eq!(abort_info.addr, addr);
    }

    #[test_case]
    fn test_data_abort_general_purpose_registers() {
        init_interrupt_handler();
        set_data_abort_handler(dummy_data_abort_handler);

        let addr: usize = 0xaeadbeef;

        unsafe {
            core::arch::asm!(
                // pc is loaded 8 bytes ahead of current instruction
                "mov r1,#0x1234",
                "mov r2,#0xdead",
                "str r1,[{addr}]",
                out("r1") _,
                out("r2") _,
                addr = in(reg) addr
            );
        }

        let abort_info = ABORT_INFO.lock();
        assert_eq!(abort_info.regs.r[1], 0x1234);
        assert_eq!(abort_info.regs.r[2], 0xdead);
    }
}
