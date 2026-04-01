use core::ptr::addr_of;

use super::{
    data_abort::data_abort_handler, gic_cpu::GicCpu, gic_dispatcher::GicDispatcher,
    interrupt_table::interrupt_table_start, reg_set::RegSet,
};
use crate::{
    alloc::{Unique, Vec},
    interrupts::Interrupt,
    spinlock::SpinLock,
};

type IrqHandler = fn(u32, &mut RegSet) -> ();

pub struct InterruptController {
    dispatcher: Unique<GicDispatcher>,
    cpu_interface: Unique<GicCpu>,
    irq_handlers: Vec<IrqHandler>,
}

pub static CONTROLLER: SpinLock<Option<InterruptController>> = SpinLock::new(None);

impl InterruptController {
    pub fn new(mut dispatcher: Unique<GicDispatcher>, mut cpu_interface: Unique<GicCpu>) -> Self {
        unsafe {
            super::interrupt_table::data_abort_handler_pointer =
                data_abort_handler as *mut extern "C" fn(*mut RegSet);
        }

        unsafe {
            super::interrupt_table::irq_handler_pointer =
                irq_handler as *mut extern "C" fn(*mut RegSet);
        }

        Self::set_high_exception_vector_address(addr_of!(interrupt_table_start) as usize);
        cpu_interface.enable_singaling_to_cpu();
        cpu_interface.set_prio_mask(GicCpu::ALLOW_ALL);
        dispatcher.enable_forarding();
        InterruptController {
            cpu_interface,
            dispatcher,
            irq_handlers: Vec::new(),
        }
    }

    fn set_high_exception_vector_address(address: usize) {
        unsafe {
            core::arch::asm!("MCR p15, 0, {}, c12, c0, 0", in(reg) address);
        }
    }

    pub fn register(&mut self, interrupt: Interrupt, handler: IrqHandler) {
        let interrupt = match interrupt {
            Interrupt::Spi(num) => num as usize + 32,
            Interrupt::Ppi(num) => num as usize + 16,
        };
        self.irq_handlers
            .resize(interrupt + 1, default_isr)
            .unwrap();
        self.irq_handlers[interrupt] = handler;
        self.dispatcher.enable_interrupt(interrupt);
    }
}

extern "C" fn irq_handler(reg_set: *mut RegSet) {
    super::without_irq(|| {
        let mut guard = CONTROLLER.lock();
        let controller = guard.as_mut().unwrap();
        let int_num = controller.cpu_interface.current_interrupt_number();
        controller.irq_handlers[int_num as usize](int_num, unsafe { &mut *reg_set });
        controller.cpu_interface.signal_end(int_num);
    });
}

fn default_isr(int_num: u32, _reg_set: &mut RegSet) {
    crate::console::println!("irq number #{}!", int_num);
}
