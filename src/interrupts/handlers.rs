use crate::{
    alloc::{Arc, Vec},
    error::{Error, Result},
    interrupts::{self, without_irq, Interrupt, RegSet},
    spinlock::SpinLock,
};

pub trait InterruptHandler {
    fn ack_interrupt(&self);
}

static HANDLERS: SpinLock<Vec<(u32, Arc<dyn InterruptHandler>)>> = SpinLock::new(Vec::new());

pub fn register_handler(handler: Arc<dyn InterruptHandler>, interrupt: Interrupt) -> Result<()> {
    let interrupt_num = match interrupt {
        Interrupt::Spi(num) => num as u32 + 32,
        _ => return Err(Error::Unsupproted),
    };
    without_irq(|| -> Result<()> {
        let mut handlers = HANDLERS.lock();
        handlers.push((interrupt_num, handler))?;
        interrupts::CONTROLLER
            .lock()
            .as_mut()
            .unwrap()
            .register(interrupt, handler_isr);
        Ok(())
    })
}

fn handler_isr(int_num: u32, _reg_set: &mut RegSet) {
    let Some(handler) = find_handler_by_intterrupt(int_num) else {
        return;
    };
    handler.ack_interrupt();
}

fn find_handler_by_intterrupt(interrupt: u32) -> Option<Arc<dyn InterruptHandler>> {
    let guard = HANDLERS.lock();
    for (curr_interrupt, handler) in &*guard {
        if *curr_interrupt == interrupt {
            return Some(Arc::<dyn InterruptHandler>::clone(handler));
        }
    }
    None
}
