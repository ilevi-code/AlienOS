use crate::mmu;
use core::cell::UnsafeCell;
use core::mem::MaybeUninit;

// The first 1GB is MMIO
const PHYS_START: usize = 0x40000000;
pub const BLOCK_SIZE: usize = 0x4000;

struct FreePage {
    next: Option<&'static mut FreePage>,
}

struct PageAllocator {
    head: *mut u8,
    free_list: Option<&'static mut FreePage>,
}

unsafe impl Sync for PageAllocator {}

struct Cell<T> {
    inner: UnsafeCell<T>,
}

unsafe impl<T> Sync for Cell<T> {}

// TODO Box like blocks with lifetimes
impl PageAllocator {
    pub fn alloc_frame(&mut self) -> usize {
        // TODO check free_list. We need temporary access to the page.
        // TODO check we do not exceed the amount of available RAM
        // TODO p2v and memset the page
        unsafe {
            let new_head = self.head.add(BLOCK_SIZE);
            let block_ptr = core::mem::replace(&mut self.head, new_head);
            block_ptr.write_bytes(0, BLOCK_SIZE);
            block_ptr as usize
        }
    }

    pub fn free_frame(&mut self, page_addr: usize) {
        let page_ptr = page_addr as *mut FreePage;
        if let Some(page) = unsafe { page_ptr.as_mut() } {
            page.next = self.free_list.take();
            self.free_list = Some(page);
        };
    }
}

static PAGE_ALLOCATOR: Cell<MaybeUninit<PageAllocator>> = Cell {
    inner: UnsafeCell::new(MaybeUninit::uninit()),
};

pub fn init() {
    let phys_head = PHYS_START + mmu::get_kernel_location().len();
    unsafe {
        (*PAGE_ALLOCATOR.inner.get()).write(PageAllocator {
            head: phys_head as *mut u8,
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
