use crate::interrupts::reg_set::RegSet;

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
    pub(super) static interrupt_table_start: u32;
    pub(super) static mut data_abort_handler_pointer: *mut extern "C" fn(*mut RegSet);
    pub(super) static mut irq_handler_pointer: *mut extern "C" fn(*mut RegSet);
    pub(super) static mut svc_handler_pointer: *mut extern "C" fn(*mut RegSet);
}
