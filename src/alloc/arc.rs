use core::marker::{PhantomData, Unsize};
use core::ops::{CoerceUnsized, Deref, DispatchFromDyn};
use core::ptr::NonNull;
use core::sync::atomic::{self, AtomicUsize, Ordering};

use crate::alloc::Box;
use crate::error::Result;

pub struct Arc<T: ?Sized> {
    ptr: NonNull<ArcInner<T>>,
    phantom: PhantomData<ArcInner<T>>,
}

pub struct ArcInner<T: ?Sized> {
    rc: AtomicUsize,
    data: T,
}

impl<T> Arc<T> {
    pub fn new(data: T) -> Result<Arc<T>> {
        // We start the reference count at 1, as that first reference is the
        // current pointer.
        let boxed = Box::new(ArcInner {
            rc: AtomicUsize::new(1),
            data,
        })?;
        Ok(Arc {
            ptr: Box::into_non_null(boxed),
            phantom: PhantomData,
        })
    }
}

unsafe impl<T: Sync + Send + ?Sized> Send for Arc<T> {}
unsafe impl<T: Sync + Send + ?Sized> Sync for Arc<T> {}

impl<T: ?Sized> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        let inner = unsafe { self.ptr.as_ref() };
        &inner.data
    }
}

impl<T: ?Sized> Clone for Arc<T> {
    fn clone(&self) -> Arc<T> {
        let inner = unsafe { self.ptr.as_ref() };
        // Using a relaxed ordering is alright here as we don't need any atomic
        // synchronization here as we're not modifying or accessing the inner
        // data.
        let old_rc = inner.rc.fetch_add(1, Ordering::Relaxed);

        if old_rc >= isize::MAX as usize {
            panic!("cloning arc caused counter to overflow")
        }

        Self {
            ptr: self.ptr,
            phantom: PhantomData,
        }
    }
}

impl<T: ?Sized> Drop for Arc<T> {
    fn drop(&mut self) {
        let inner = unsafe { self.ptr.as_ref() };
        if inner.rc.fetch_sub(1, Ordering::Release) != 1 {
            return;
        }
        // This fence is needed to prevent reordering of the use and deletion
        // of the data.
        atomic::fence(Ordering::Acquire);
        // This is safe as we know we have the last pointer to the `ArcInner`
        // and that its pointer is valid.
        unsafe {
            Box::from_non_null(self.ptr);
        }
    }
}

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<Arc<U>> for Arc<T> {}

impl<T: ?Sized, U: ?Sized> DispatchFromDyn<Arc<U>> for Arc<T> where T: Unsize<U> {}
