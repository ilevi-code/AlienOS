use crate::error::Result;
use alien_traits::Pod;

pub struct User<T>(pub(super) T);

impl<T: Clone> Clone for User<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: Copy> Copy for User<T> {}

impl User<u8> {
    pub fn write(&mut self, val: u8) -> Result<()> {
        super::copy_byte_to_user(self, val)
    }

    pub fn load(&self) -> Result<u8> {
        super::copy_byte_from_user(self)
    }
}

pub trait AsUserBytes {
    fn as_user_bytes(&mut self) -> &mut [User<u8>];
}

impl<T: ?Sized + Pod> AsUserBytes for T {
    fn as_user_bytes(&mut self) -> &mut [User<u8>] {
        unsafe {
            core::slice::from_raw_parts_mut(
                core::ptr::from_mut(self) as *mut User<u8>,
                core::mem::size_of_val(self),
            )
        }
    }
}
