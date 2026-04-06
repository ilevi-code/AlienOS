use crate::{
    alloc::{Arc, Box},
    error::Result,
    fs::File,
    sys::User,
};

#[allow(unused)]
pub trait CharDev {
    fn read(&self, buf: &mut [User<u8>]) -> Result<()>;

    // TODO change to &[User<u8>]
    fn write(&self, buf: &[u8]) -> Result<()>;

    fn open(self: Arc<Self>) -> Result<Box<dyn File>>;
}
