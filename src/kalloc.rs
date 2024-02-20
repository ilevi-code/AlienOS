use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use crate::mmu;

// The first 1GB is MMIO
const PHYS_START: usize = 0x40000000;
pub const BLOCK_SIZE: usize = 0x4000;

struct FreePage {
    next: Option<*mut FreePage>,
}

struct PageAllocator {
    head: usize,
    free_list: Option<*mut FreePage>,
}

unsafe impl Sync for PageAllocator {}

struct Cell<T> {
    inner: UnsafeCell<T>,
}

unsafe impl<T> Sync for Cell<T> {}

impl PageAllocator {
    pub fn alloc_frame(&mut self) -> usize {
        // TODO check free_list. We need temporary access to the page.
        // TODO check we do not exceed the amount of available RAM
        let old_head = self.head;
        self.head = old_head + BLOCK_SIZE;
        old_head
    }

    pub unsafe fn free_frame(&mut self, page: usize) {
        let page = page as *mut FreePage;
        (*page).next = self.free_list;
        self.free_list = Some(page);
    }
}

static PAGE_ALLOCATOR: Cell<MaybeUninit<PageAllocator>> = Cell {
    inner: UnsafeCell::new(MaybeUninit::uninit()),
};

pub fn init() {
    let phys_head = PHYS_START + mmu::get_kernel_location().len();
    unsafe {
        (*PAGE_ALLOCATOR.inner.get()).write(PageAllocator {
            head: phys_head,
            free_list: None,
        });
    }
}

pub fn alloc_frame() -> usize {
    unsafe {
        (*PAGE_ALLOCATOR.inner.get())
            .assume_init_mut()
            .alloc_frame()
    }
}
