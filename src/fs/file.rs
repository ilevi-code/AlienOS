use crate::{error::Result, sys::User};

pub enum SeekFrom {
    Start(usize),
    #[allow(unused)]
    Current(usize),
}

pub trait File {
    fn read(&mut self, buf: &mut [User<u8>]) -> Result<()>;

    fn seek(&mut self, position: SeekFrom) -> Result<()>;

    fn write(&mut self, buf: &[User<u8>]) -> Result<()>;
}
