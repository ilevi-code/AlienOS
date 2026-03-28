use core::{cell::UnsafeCell, ptr::NonNull};

pub trait PerCpuable {}

impl PerCpuable for u32 {}
impl<T> PerCpuable for *mut T {}
impl<T> PerCpuable for NonNull<T> {}
impl<T: PerCpuable> PerCpuable for Option<T> {}

pub struct PerCpu<T: PerCpuable>(UnsafeCell<T>);

// TODO Safety
unsafe impl<T: PerCpuable> Sync for PerCpu<T> {}

impl<T: PerCpuable> PerCpu<T> {
    pub const fn new(val: T) -> Self {
        Self(UnsafeCell::new(val))
    }

    // pub fn borrow_mut(&self) -> RefMut<'_, T> {
    //     self.0.borrow_mut()
    // }

    // pub fn try_borrow_mut(&self) -> Option<RefMut<'_, T>> {
    //     self.0.try_borrow_mut().ok()
    // }

    pub fn as_ptr(&self) -> *mut T {
        self.0.get()
    }

    // pub fn replace(&self, val: T) -> T {
    //     self.0.replace(val)
    // }
    pub fn set(&self, val: T) {
        unsafe { self.0.get().write(val) }
    }
}

impl<T: PerCpuable> PerCpu<T>
where
    T: Copy,
{
    pub fn get(&self) -> T {
        unsafe { *self.0.get() }
    }

    pub fn replace(&self, val: T) -> T {
        let old = self.get();
        self.set(val);
        old
    }
}

#[macro_export]
macro_rules! per_cpu {
    ($name:ident : $ty:ty = $val:expr) => {
        static_assertions::assert_impl_all!($ty: $crate::mmu::PerCpuable);

        #[link_section = "per_cpu"]
        pub static $name: $crate::mmu::PerCpu<$ty> = $crate::mmu::PerCpu::new($val);
    };
}
