use crate::{
    alloc::{Arc, Vec},
    drivers::block::Device,
    error::Result,
    spinlock::SpinLock,
};

static DISKS: SpinLock<Vec<Arc<dyn Device>>> = SpinLock::new(Vec::new());

pub fn register_disk(disk: Arc<dyn Device>) -> Result<()> {
    let mut guard = DISKS.lock();
    guard.push(disk)?;
    Ok(())
}

pub fn get_disk_by_id(id: usize) -> Option<Arc<dyn Device>> {
    let disks = DISKS.lock();
    let disk = &(disks.get(id)?);
    Some(Arc::clone(disk))
}
