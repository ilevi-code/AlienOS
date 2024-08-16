/// Use to represent a pointer to a **physical** memory
pub struct Phys<T>(*mut T);

impl<T> Phys<T> {
    /// Obtain a reference to the pointed object.
    ///
    /// # SAFETY
    /// The caller must ensure that the physical is accesible for reads operations.
    unsafe fn get(&self) -> &mut T {
        &mut *self.0
    }

    /// Obtain a mutable reference to the pointed object.
    ///
    /// # SAFETY
    /// The caller must ensure that the physical is accesible to both read and write operations.
    unsafe fn get_mut(&self) -> &mut T {
        &mut *self.0
    }

    pub fn addr(&self) -> usize {
        self.0 as usize
    }

    pub fn cast<U>(self) -> Phys<U> {
        Phys::<U> {
            0: self.0 as *mut U,
        }
    }
}

impl<T> From<usize> for Phys<T> {
    fn from(value: usize) -> Phys<T> {
        Self { 0: value as *mut T }
    }
}

impl<T> From<*mut T> for Phys<T> {
    fn from(ptr: *mut T) -> Phys<T> {
        Self { 0: ptr }
    }
}
