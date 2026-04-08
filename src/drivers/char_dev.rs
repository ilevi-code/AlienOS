use crate::{
    alloc::{Arc, Box, Vec},
    error::Result,
    fs::File,
    spinlock::SpinLock,
    sys::User,
};

use alien_derive::IntEnum;

#[allow(unused)]
pub trait CharDev {
    fn read(&self, buf: &mut [User<u8>]) -> Result<()>;

    fn write(&self, buf: &[User<u8>]) -> Result<()>;

    fn open(self: Arc<Self>) -> Result<Box<dyn File>>;
}

#[derive(IntEnum)]
pub enum Major {
    Pl011 = 1,
    #[default]
    Unknown,
}

static CHAR_DEVS: SpinLock<Vec<(u32, Arc<dyn CharDev>)>> = SpinLock::new(Vec::new());

pub fn char_dev_register(dev: Arc<dyn CharDev>, major: Major) -> Result<()> {
    let mut devs = CHAR_DEVS.lock();
    devs.push((major.into(), dev))?;
    Ok(())
}

pub fn char_dev_lookup(major: u32) -> Option<Arc<dyn CharDev>> {
    let devs = CHAR_DEVS.lock();
    for (curr_major, dev) in &*devs {
        if *curr_major == major {
            return Some(Arc::clone(dev));
        }
    }
    None
}
