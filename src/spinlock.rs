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
            let locked = self.try_lock();
            if !locked {
                core::hint::spin_loop()
            } else {
                break;
            }
        }
    }

    #[inline]
    pub fn try_lock(&self) -> bool {
        self.state
            .compare_exchange(UNLOCKED, LOCKED, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
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

    pub fn try_lock(&self) -> Option<SpinLockGuard<'_, T>> {
        unsafe {
            if self.inner.try_lock() {
                Some(SpinLockGuard::new(self))
            } else {
                None
            }
        }
    }
}

unsafe impl<T> Sync for SpinLock<T> {}
