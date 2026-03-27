use crate::sys::{Errno, User};

pub trait File {
    fn read(&mut self, buf: User<&[u8]>) -> core::result::Result<(), Errno>;
}
