use core::cell::{RefCell, RefMut};

pub struct PerCpu<T>(RefCell<T>);

unsafe impl<T> Sync for PerCpu<T> {}

impl<T> PerCpu<T> {
    pub const fn new(val: T) -> Self {
        Self(RefCell::new(val))
    }

    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        self.0.borrow_mut()
    }

    pub fn try_borrow_mut(&self) -> Option<RefMut<'_, T>> {
        self.0.try_borrow_mut().ok()
    }

    pub fn as_ptr(&self) -> *mut T {
        self.0.as_ptr()
    }

    pub fn replace(&self, val: T) -> T {
        self.0.replace(val)
    }
}

impl<T> PerCpu<T>
where
    T: Copy,
{
    pub fn get(&self) -> T {
        *self.0.borrow()
    }
}

#[macro_export]
macro_rules! per_cpu {
    ($name:ident : $ty:ty = $val:expr) => {
        static_assertions::assert_impl_all!($ty: Copy, Clone);

        #[link_section = "per_cpu"]
        pub static $name: $crate::mmu::PerCpu<$ty> = $crate::mmu::PerCpu::new($val);
    };
}
