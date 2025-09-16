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
        Ok(unsafe { Box::<MaybeUninit<T>>::assume_init(uninit) })
    }

    #[must_use]
    pub(crate) fn zeroed() -> Result<Box<T>> {
        // SAFETY:
        // Layout is of a valid type, and initialization is encofrced with by zeroing
        let ptr = unsafe { ALLOCATOR.alloc_zeroed(Layout::new::<T>()) };
        let ptr = NonNull::new(ptr.cast::<T>()).ok_or(Error::OutOfMem)?;
        Ok(Box(ptr))
    }

    pub(crate) fn into_non_null(b: Self) -> NonNull<T> {
        let b = core::mem::ManuallyDrop::new(b);
        b.0
    }

    /// # Safety
    ///
    /// The pointer must point to a block of memory allocated by the global allocator.
    pub(crate) unsafe fn from_non_null(ptr: NonNull<T>) -> Self {
        Box(ptr)
    }
}

impl<T> From<Box<T>> for NonNull<T> {
    fn from(val: Box<T>) -> Self {
        let ptr = val.0;
        core::mem::forget(val);
        ptr
    }
}

impl<T> Box<MaybeUninit<T>> {
    /// # Safety:
    /// Must be called only when `self` has be initialized
    pub(crate) unsafe fn assume_init(this: Self) -> Box<T> {
        let this = core::mem::ManuallyDrop::new(this);
        Box(this.0.cast::<T>())
        // Not dropping this, since ownership was transferred to the returned Box
    }
}

impl<T: ?Sized> Drop for Box<T> {
    fn drop(&mut self) {
        // SAFETY:
        // By definition of Box, the pointer is valid for read and writes, and non-null, and
        // uniquely owned.
        // The pointer is also properly aligned, since the layout was constructed from T.
        unsafe { core::ptr::drop_in_place(self.0.as_ptr()) };
        let layout = Layout::for_value(unsafe { self.0.as_ref() });
        // SAFETY:
        // 1. Same pointer we got from alloc
        // 2. Same layout, since it is created from the value itself
        unsafe {
            ALLOCATOR.dealloc(self.0.cast::<u8>().as_ptr(), layout);
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
