use crate::{
    alloc::{Arc, Box},
    error::Result,
    fs::File,
    sys::User,
};

#[allow(unused)]
pub trait CharDev {
    fn read(&self, buf: &mut [User<u8>]) -> Result<()>;

    fn write(&self, buf: &[User<u8>]) -> Result<()>;

    fn open(self: Arc<Self>) -> Result<Box<dyn File>>;
}
