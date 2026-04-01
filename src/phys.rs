use crate::memory_model;

/// Use to represent a pointer to a **physical** memory
#[cfg_attr(test, derive(PartialEq))]
pub struct Phys<T: ?Sized>(*const T);

#[cfg_attr(test, derive(PartialEq))]
pub struct PhysMut<T: ?Sized>(*mut T);

impl<T> Phys<T> {
    pub fn addr(&self) -> usize {
        self.0 as usize
    }

    pub unsafe fn byte_add(self, count: usize) -> Self {
        Self(self.0.byte_add(count))
    }

    pub fn into_virt(self) -> *const T {
        unsafe { self.0.byte_add(memory_model::PHYS_TO_VIRT) }
    }
}

impl<T: ?Sized> Phys<T> {
    pub fn from_virt(ptr: *const T) -> Self {
        Self(unsafe { ptr.byte_sub(memory_model::PHYS_TO_VIRT) })
    }
}

impl<T> PhysMut<T> {
    pub fn addr(&self) -> usize {
        self.0.addr()
    }

    pub fn from_virt(ptr: *mut T) -> Self {
        Self(unsafe { ptr.byte_sub(memory_model::PHYS_TO_VIRT) })
    }
}

impl<T: ?Sized> PhysMut<T> {
    pub fn into_virt(self) -> *mut T {
        unsafe { self.0.byte_add(memory_model::PHYS_TO_VIRT) }
    }
}

impl<T> Phys<[T]> {
    pub fn addr(&self) -> usize {
        (self.0 as *mut T) as usize
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn with_addr(self, addr: usize) -> *const [T] {
        self.0.with_addr(addr)
    }
}

impl<T> From<*const [T]> for Phys<[T]> {
    fn from(value: *const [T]) -> Self {
        // TODO make phys immutable pointer
        Self(value as *mut [T])
    }
}

impl<T> From<usize> for Phys<T> {
    fn from(value: usize) -> Phys<T> {
        Self(value as *const T)
    }
}

impl<T> From<usize> for PhysMut<T> {
    fn from(value: usize) -> Self {
        Self(value as *mut T)
    }
}

impl<T> From<*mut T> for Phys<T> {
    fn from(ptr: *mut T) -> Self {
        Self(ptr)
    }
}

impl<T> From<*mut T> for PhysMut<T> {
    fn from(ptr: *mut T) -> Self {
        Self(ptr)
    }
}

impl<T> core::fmt::Debug for Phys<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("Phys").field(&self.0).finish()
    }
}

impl<T> Clone for Phys<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Phys<T> {}
