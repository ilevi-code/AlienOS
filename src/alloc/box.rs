use crate::error::Error;
use crate::{error::Result, heap::ALLOCATOR};

use core::alloc::{GlobalAlloc, Layout};
use core::ops::{Deref, DerefMut};
use core::{mem::MaybeUninit, ptr::NonNull};

pub(crate) struct Box<T: ?Sized>(NonNull<T>);

impl<T> Box<T> {
    #[must_use]
    pub(crate) fn new_uninit() -> Result<Box<MaybeUninit<T>>> {
        // SAFETY:
        // Layout is of a valid type, and initialization is encofrced with `MaybeUninit`
        let ptr = unsafe { ALLOCATOR.alloc(Layout::new::<MaybeUninit<T>>()) };
        let ptr = NonNull::new(ptr.cast::<MaybeUninit<T>>()).ok_or(Error::OutOfMem)?;
        Ok(Box(ptr))
    }

    #[must_use]
    pub(crate) fn new(value: T) -> Result<Box<T>> {
        let mut uninit = Self::new_uninit()?;
        // SAFETY:
        // Pointer is convertible to refernece, since it was allocated and verified to be non-null.
        let mut_ref = unsafe { uninit.0.as_mut() };
        mut_ref.write(value);
        // SAFETY:
        // The value has been initialized
        Ok(unsafe { uninit.init() })
    }

    #[must_use]
    pub(crate) fn zeroed() -> Result<Box<T>> {
        // SAFETY:
        // Layout is of a valid type, and initialization is encofrced with by zeroing
        let ptr = unsafe { ALLOCATOR.alloc_zeroed(Layout::new::<T>()) };
        let ptr = NonNull::new(ptr.cast::<T>()).ok_or(Error::OutOfMem)?;
        Ok(Box(ptr))
    }
}

impl<T> Into<NonNull<T>> for Box<T> {
    fn into(self) -> NonNull<T> {
        let ptr = self.0;
        core::mem::forget(self);
        ptr
    }
}

impl<T> Box<MaybeUninit<T>> {
    /// # Safety:
    /// Must be called only when `self` has be initialized
    pub(crate) unsafe fn init(self) -> Box<T> {
        Box(self.0.cast::<T>())
    }
}

impl<T> Drop for Box<T> {
    fn drop(&mut self) {
        // SAFETY:
        // Same pointer we got from alloc, and same layout.
        unsafe {
            ALLOCATOR.dealloc(
                self.0.cast::<u8>().as_ptr(),
                Layout::new::<MaybeUninit<T>>(),
            );
        }
    }
}

impl<T> Deref for Box<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}

impl<T> DerefMut for Box<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.as_mut() }
    }
}

impl<T> core::fmt::Debug for Box<T>
where
    T: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let inner = unsafe { self.0.as_ref() };
        f.debug_tuple("Box").field(inner).finish()
    }
}
