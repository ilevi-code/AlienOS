use core::alloc::{GlobalAlloc, Layout};

use crate::{
    console::println,
    error::{Error, Result},
    spinlock::SpinLock,
};

use super::kern_allocator::KernAlloctor;

pub(crate) struct GlobalKernAllocator(SpinLock<Option<KernAlloctor>>);

unsafe impl GlobalAlloc for GlobalKernAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match self
            .0
            .lock()
            .as_mut()
            .expect("Heap should be initilized before alloc")
            .alloc(layout)
        {
            Ok(ptr) => ptr,
            _ => core::ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.0
            .lock()
            .as_mut()
            // Also, this indicates free before alloc, since alloc should have paniced first
            .expect("Heap should be initilized before free")
            .dealloc(ptr, layout)
    }
}

pub(crate) static ALLOCATOR: GlobalKernAllocator = GlobalKernAllocator(SpinLock::new(None));

pub fn init(kern_end: usize, ram_end: usize) {
    let mut scale = "";
    let mut size = ram_end - kern_end;
    for current_scale in ["", "K", "M", "G"] {
        scale = current_scale;
        if size < 1024 {
            break;
        }
        size /= 1024;
    }
    println!("kalloc init: {:x}, size {}{}B", kern_end, size, scale);
    *ALLOCATOR.0.lock() = Some(KernAlloctor::new(kern_end as *mut u8, ram_end as *mut u8));
}

pub fn alloc<T>() -> Result<*mut T> {
    let ptr = ALLOCATOR
        .0
        .lock()
        .as_mut()
        .expect("Heap should be initlized before alloc")
        .alloc(Layout::new::<T>())?
        .cast::<T>();
    if ptr.is_null() {
        return Err(Error::OutOfMem);
    }
    Ok(ptr)
}
