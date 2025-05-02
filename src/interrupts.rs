use core::ptr::addr_of;
extern "C" {
    static interrupt_table_start: u32;
    static interrupt_table_end: u32;
    static mut data_abort_handler_pointer: *mut extern "C" fn();
    // static mut data_abort_handler_pointer: usize;
}

core::arch::global_asm!(
    ".section interrupt_table, \"ax\"",
    ".global interrupt_table_start",
    "interrupt_table_start:",
    "",
    "nop",
    "nop",
    "nop",
    "nop",
    "ldr pc, data_abort_handler_pointer",
    ".global data_abort_handler_pointer",
    "data_abort_handler_pointer:",
    ".word 0x0",
    "",
    "",
    ".global interrupt_table_end",
    "interrupt_table_end:",
    ".global _data_abort_handler",
    "_data_abort_handler:",
    "sub lr, lr, #4",
    "srsdb #23!", // push LR_abt and CPSR_abt to the stack.
    "push {{r0-r12}}",
    "sub r0, lr, #4",
    "bl data_abort_handler",
    "pop {{r0-r12}}",
    "rfeia sp!", // load LR and SPSR from the stack
);
extern "C" {
    fn _data_abort_handler();
}

fn read_fault_register() -> usize {
    let fault_address: usize;
    unsafe {
        core::arch::asm!("MRC p15, 0, {}, c6, c0, 0", out(reg) fault_address);
    }
    fault_address
}

#[unsafe(no_mangle)]
extern "C" fn data_abort_handler(fault_instruction_addres: usize) {
    crate::console::println!(
        "fault acessing address 0x{:x} from 0x{:x}",
        read_fault_register(),
        fault_instruction_addres,
    );
}

fn set_high_exception_vector_address(address: usize) {
    unsafe {
        core::arch::asm!("MCR p15, 0, {}, c12, c0, 0", in(reg) address);
    }
}

pub(crate) fn init_interrupt_handler() {
    unsafe {
        data_abort_handler_pointer = _data_abort_handler as *mut extern "C" fn();
    }
    unsafe {
        let p: *const u32 = &interrupt_table_start;
        crate::console::println!("table start at 0x{:?}", p);
    }

    // TODO setup stack for:
    // abort (mode 0b10111)
    // FIQ (mode 0b10001)
    // IRQ (mode 0b10010)
    //
    // ldr r0, =_stack_for_mode
    // msr CPSR_c, #0x18    ; Switch to IRQ mode
    // mov sp, r0
    // msr CPSR_c, #0x13    ; switch back to SVC mode (kernel mode)
    unsafe {
        core::arch::asm!("msr CPSR_c, #0x17");
        core::arch::asm!("msr CPSR_c, #0x13");
    }

    set_high_exception_vector_address((unsafe { &interrupt_table_start } as *const u32) as usize);

    unsafe {
        let addr: usize = 0xaeadbeef;
        core::arch::asm!("str r1,[{addr}]", addr = in(reg) addr);
    }
}
