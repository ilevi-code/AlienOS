/// Use to represent a pointer to a **physical** memory
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Phys<T>(*mut T);

impl<T> Phys<T> {
    pub fn addr(&self) -> usize {
        self.0 as usize
    }

    pub fn cast<U>(self) -> Phys<U> {
        Phys::<U>(self.0 as *mut U)
    }
}

impl<T> From<usize> for Phys<T> {
    fn from(value: usize) -> Phys<T> {
        Self(value as *mut T)
    }
}

impl<T> From<*mut T> for Phys<T> {
    fn from(ptr: *mut T) -> Phys<T> {
        Self(ptr)
    }
}
