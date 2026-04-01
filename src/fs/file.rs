use core::slice;

use crate::{error::Result, sys::User};

pub trait File {
    fn read(&mut self, buf: &mut [User<u8>]) -> Result<()>;
}

pub fn read_into<T>(file: &mut dyn File, data: &mut T) -> Result<()> {
    let buf = unsafe {
        slice::from_raw_parts_mut(
            core::ptr::from_mut(data) as *mut User<u8>,
            core::mem::size_of::<T>(),
        )
    };
    file.read(buf)
}

pub fn read_into_slice<T>(file: &mut dyn File, data: &mut [T]) -> Result<()> {
    let buf = unsafe {
        slice::from_raw_parts_mut(
            core::ptr::from_mut(data) as *mut User<u8>,
            core::mem::size_of_val(data),
        )
    };
    file.read(buf)
}
