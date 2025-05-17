use crate::error::Error;
use crate::{error::Result, heap::ALLOCATOR};
use core::alloc::{GlobalAlloc, Layout};
use core::ptr;

pub(crate) struct Vec<T> {
    buf: *mut T,
    capacity: usize,
    length: usize,
}

impl<T> Vec<T> {
    pub(crate) fn new() -> Self {
        Self {
            buf: ptr::null_mut(),
            capacity: 0,
            length: 0,
        }
    }

    pub(crate) fn layout_of_capacity(capacity: usize) -> Result<Layout> {
        Ok(Layout::from_size_align(
            size_of::<T>() * capacity,
            align_of::<T>(),
        )?)
    }

    pub(crate) fn extend_from_slice(&mut self, src: &[T]) -> Result<()>
    where
        T: Copy,
    {
        self.grow(src.len())?;
        let end = unsafe { self.buf.add(self.length) };
        unsafe { end.copy_from_nonoverlapping(src.as_ptr(), src.len()) };
        self.length += src.len();
        Ok(())
    }

    fn grow(&mut self, spare: usize) -> Result<()> {
        if self.length + spare < self.capacity {
            return Ok(());
        }
        let mut new_cap = self.capacity;
        if new_cap == 0 {
            new_cap = 8;
        }
        while new_cap < self.length + spare {
            new_cap *= 2;
        }
        let new = if self.buf.is_null() {
            // SAFETY:
            // Layout is of a valid type, and initialization is encofrced with `MaybeUninit`
            unsafe { ALLOCATOR.alloc(Self::layout_of_capacity(new_cap)?) }
        } else {
            // SAFETY:
            // * `buf` was allocator with `ALLOCATOR`
            // * layout is the same
            // new_size is greater than zero
            unsafe {
                ALLOCATOR.realloc(
                    self.buf.cast::<u8>(),
                    Self::layout_of_capacity(self.capacity)?,
                    new_cap * size_of::<T>(),
                )
            }
        };
        if new.is_null() {
            return Err(Error::OutOfMem);
        }
        self.buf = new.cast::<T>();
        self.capacity = new_cap;
        Ok(())
    }

    fn move_into(&mut self, new: *mut T) {
        unsafe {
            new.copy_from_nonoverlapping(self.buf, self.length);
        }
    }

    fn deallocate(&mut self) {
        if !self.buf.is_null() {
            unsafe {
                ALLOCATOR.dealloc(
                    self.buf.cast::<u8>(),
                    Self::layout_of_capacity(self.capacity).unwrap(),
                );
            }
        }
    }

    pub(crate) fn as_mut_ptr(&mut self) -> *mut T {
        self.buf
    }
}
