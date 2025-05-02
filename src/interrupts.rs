use core::ptr::addr_of;
extern "C" {
    static interrupt_table_start: u32;
    static mut data_abort_handler_pointer: *mut extern "C" fn(usize);
}

core::arch::global_asm!(
    ".section interrupt_table, \"ax\"",
    ".global interrupt_table_start",
    "interrupt_table_start:",
    "",
    "nop",                   // reset handler
    "nop",                   // undefined instruction handler
    "nop",                   // svc handler
    "nop",                   // prefetch abort
    "b _data_abort_handler", // data abort
    "nop",                   // unused
    "nop",                   // IRQ
    "nop",                   // FIQ
    "",
    "",
    ".global _data_abort_handler",
    "_data_abort_handler:",
    "sub lr, lr, #4",
    "srsdb #23!", // push LR_abt and CPSR_abt to the stack.
    "push {{r0-r12}}",
    "sub r0, lr, #4",
    "ldr r1, data_abort_handler_pointer",
    "blx r1",
    "pop {{r0-r12}}",
    "rfeia sp!", // load LR and SPSR from the stack
    ".global data_abort_handler_pointer",
    "data_abort_handler_pointer:",
    ".word 0x0",
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

fn set_data_abort_handler(handler: extern "C" fn(usize)) {
    unsafe {
        data_abort_handler_pointer = handler as *mut extern "C" fn(usize);
    }
}

pub(crate) fn init_interrupt_handler() {
    set_data_abort_handler(data_abort_handler);
    // TODO setup stack for:
    // abort (mode 0b10111)
    // FIQ (mode 0b10001)
    // IRQ (mode 0b10010)
    // currently, SP_abrt is 0, so the stack grow down from 0xffff_ffff.
    set_high_exception_vector_address(addr_of!(interrupt_table_start) as usize);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spinlock::SpinLock;

    static ABORT_INFO: SpinLock<(usize, usize)> = SpinLock::new((0, 0));

    extern "C" fn dummy_data_abort_handler(pc: usize) {
        let mut abort_info = ABORT_INFO.lock();
        abort_info.0 = pc;
        abort_info.1 = read_fault_register();
    }

    #[test_case]
    fn test_data_abort() {
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

        let abort_info: (usize, usize) = *ABORT_INFO.lock();
        assert_eq!(abort_info.0, pc);
        assert_eq!(abort_info.1, addr);
    }
}
