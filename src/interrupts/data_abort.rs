use crate::interrupts::reg_set::RegSet;

pub(super) fn read_fault_register() -> usize {
    let fault_address: usize;
    unsafe {
        core::arch::asm!("MRC p15, 0, {}, c6, c0, 0", out(reg) fault_address);
    }
    fault_address
}

#[no_mangle]
pub(super) extern "C" fn data_abort_handler(reg_set: *mut RegSet) {
    crate::console::println!(
        "fault acessing address 0x{:x} from 0x{:x}",
        read_fault_register(),
        unsafe { &*reg_set }.lr,
    );
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

    fn set_data_abort_handler(handler: extern "C" fn(*mut RegSet)) {
        unsafe {
            super::super::interrupt_table::data_abort_handler_pointer =
                handler as *mut extern "C" fn(*mut RegSet);
        }
    }

    #[test_case]
    fn test_data_abort_reported_fault_address() {
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
