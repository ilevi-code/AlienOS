use crate::{
    alloc::{Arc, Vec},
    drivers::block::Device,
    error::{Error, Result},
    interrupts::{self, without_irq, Interrupt, RegSet},
    sched::wakeup,
    spinlock::SpinLock,
};

static DISKS: SpinLock<Vec<(u32, Arc<dyn Device>)>> = SpinLock::new(Vec::new());

pub fn register_disk(disk: Arc<dyn Device>, interrupt: Interrupt) -> Result<()> {
    let interrupt_num = match interrupt {
        Interrupt::Spi(num) => num as u32 + 32,
        _ => return Err(Error::Unsupproted),
    };
    without_irq(|| -> Result<()> {
        let mut guard = DISKS.lock();
        guard.push((interrupt_num, disk))?;
        interrupts::CONTROLLER
            .lock()
            .as_mut()
            .unwrap()
            .register(interrupt, disk_isr);
        Ok(())
    })
}

fn disk_isr(int_num: u32, _reg_set: &mut RegSet) {
    let Some(disk) = find_disk_by_intterrupt(int_num) else {
        return;
    };
    disk.ack_interrupt();
    wakeup(Arc::<dyn Device>::as_ptr(&disk).addr());
}

fn find_disk_by_intterrupt(interrupt: u32) -> Option<Arc<dyn Device>> {
    let guard = DISKS.lock();
    for (curr_interrupt, disk) in &*guard {
        if *curr_interrupt == interrupt {
            return Some(Arc::<dyn Device>::clone(disk));
        }
    }
    None
}

pub fn get_disk_by_id(id: usize) -> Option<Arc<dyn Device>> {
    let disks = DISKS.lock();
    let disk = &(disks.get(id)?.1);
    Some(Arc::clone(disk))
}
