use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicU8, Ordering},
};

const UNLOCKED: u8 = 0;
const LOCKED: u8 = 1;

struct SpinLockImpl {
    state: AtomicU8,
}

impl SpinLockImpl {
    pub const fn new() -> Self {
        Self {
            state: AtomicU8::new(UNLOCKED),
        }
    }

    pub fn lock(&self) {
        loop {
            match self.state.compare_exchange(
                UNLOCKED,
                LOCKED,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => return,
                Err(_) => core::hint::spin_loop(),
            }
        }
    }

    #[inline]
    pub unsafe fn unlock(&self) {
        self.state.swap(UNLOCKED, Ordering::Release);
    }
}

pub(crate) struct SpinLock<T> {
    inner: SpinLockImpl,
    data: UnsafeCell<T>,
}

pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<'mutex, T> SpinLockGuard<'mutex, T> {
    unsafe fn new(lock: &'mutex SpinLock<T>) -> SpinLockGuard<'mutex, T> {
        Self { lock }
    }
}

impl<T> Drop for SpinLockGuard<'_, T> {
    fn drop(&mut self) {
        unsafe { self.lock.inner.unlock() }
    }
}

impl<T> Deref for SpinLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for SpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> !Send for SpinLockGuard<'_, T> {}
impl<T> !Sync for SpinLockGuard<'_, T> {}

impl<T> SpinLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            inner: SpinLockImpl::new(),
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        unsafe {
            self.inner.lock();
            SpinLockGuard::new(self)
        }
    }
}

unsafe impl<T> Sync for SpinLock<T> {}
