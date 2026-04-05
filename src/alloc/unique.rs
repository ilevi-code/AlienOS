use core::{
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

pub struct Unique<T> {
    ptr: NonNull<T>,
}

impl<T> Unique<T> {
    pub const fn from_non_null(ptr: NonNull<T>) -> Self {
        Self { ptr }
    }
}

impl<T> From<NonNull<T>> for Unique<T> {
    fn from(ptr: NonNull<T>) -> Self {
        Self::from_non_null(ptr)
    }
}

impl<T> Deref for Unique<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> DerefMut for Unique<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut() }
    }
}
