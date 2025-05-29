use core::ptr::addr_of;

use super::{
    data_abort::data_abort_handler, gic_cpu::GicCpu, gic_dispatcher::GicDispatcher,
    interrupt_table::interrupt_table_start, reg_set::RegSet,
};
use crate::{
    alloc::{Box, Vec},
    drivers::virtio_blk::Unique,
};

type IrqHandler = Box<dyn Fn() -> ()>;

struct InterruptController {
    gic_cpu: Unique<GicCpu>,
    gic_dispatcher: Unique<GicDispatcher>,
    irq_handlers: Vec<IrqHandler>,
}

extern "C" fn irq_handler(reg_set: *mut RegSet) {
    let gicc: GicCpu;
    let int_num = gicc.current_interrupt_number();
    crate::console::println!("irq number #{}!\n", int_num);
    if int_num == timer::VirtualCounter::irq_id() {
        let mut timer = timer::VirtualCounter;
        timer.arm(timer.frequency());
    } else if int_num == 79 {
        match *disk_handler.lock() {
            Some(handler) => handler(),
            None => crate::console::println!("no disk handler"),
        }
    }
    gicc.signal_end(int_num);
}

pub(crate) fn svc_handler(reg_set: *mut RegSet) {
    crate::console::println!("syscall!");
    crate::semihosting::shutdown(0);
}

impl InterruptController {
    fn new(gic_cpu: Unique<GicCpu>, gic_dispatcher: Unique<GicDispatcher>) -> Self {
        unsafe {
            super::interrupt_table::data_abort_handler_pointer =
                data_abort_handler as *mut extern "C" fn(*mut RegSet);
        }
        unsafe {
            super::interrupt_table::svc_handler_pointer =
                svc_handler as *mut extern "C" fn(*mut RegSet);
        }

        unsafe {
            super::interrupt_table::irq_handler_pointer =
                irq_handler as *mut extern "C" fn(*mut RegSet);
        }
        Self::set_high_exception_vector_address(addr_of!(interrupt_table_start) as usize);
        InterruptController {
            gic_cpu,
            gic_dispatcher,
            irq_handlers: Vec::new(),
        }
    }

    fn set_high_exception_vector_address(address: usize) {
        unsafe {
            core::arch::asm!("MCR p15, 0, {}, c12, c0, 0", in(reg) address);
        }
    }
}
