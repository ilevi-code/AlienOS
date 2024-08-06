use crate::console::println;
use crate::mmu;
use core::cell::UnsafeCell;
use core::mem::MaybeUninit;

// The first 1GB is MMIO
const PHYS_START: usize = 0x40000000;
pub const BLOCK_SIZE: usize = 0x4000;

struct FreePage {
    next: *mut FreePage,
}

struct PageAllocator {
    head: *mut u8,
    end: *mut u8,
    free_list: *mut FreePage,
}

unsafe impl Sync for PageAllocator {}

struct Cell<T> {
    inner: UnsafeCell<T>,
}

unsafe impl<T> Sync for Cell<T> {}

// TODO Box like blocks with lifetimes
impl PageAllocator {
    pub fn alloc_frame(&mut self) -> usize {
        return match unsafe { self.free_list.as_mut() } {
            Some(list_head) => {
                let (poped, new_head) = Self::pop_free_list(list_head);
                self.free_list = new_head;
                poped
            }
            None => self.advance_head(),
        };
    }

    fn pop_free_list(list_head: &mut FreePage) -> (usize, *mut FreePage) {
        let old_head: *mut FreePage = list_head;
        (old_head as usize, list_head.next)
    }

    fn advance_head(&mut self) -> usize {
        unsafe {
            let new_head = self.head.add(BLOCK_SIZE);
            if self.end.offset_from(new_head) < BLOCK_SIZE as isize {
                panic!("Out of memory!");
            }
            let block_ptr = core::mem::replace(&mut self.head, new_head);
            block_ptr.write_bytes(0, BLOCK_SIZE);
            block_ptr as usize
        }
    }

    pub fn free_frame(&mut self, page_addr: usize) {
        let page_ptr = page_addr as *mut FreePage;
        if let Some(page) = unsafe { page_ptr.as_mut() } {
            page.next = self.free_list;
            self.free_list = page;
        };
    }
}

static PAGE_ALLOCATOR: Cell<MaybeUninit<PageAllocator>> = Cell {
    inner: UnsafeCell::new(MaybeUninit::uninit()),
};

pub fn init(kern_end: usize, ram_end: usize) {
    let mut scale = "";
    let mut size = ram_end - kern_end;
    for current_scale in ["", "K", "M", "G"] {
        scale = current_scale;
        if size / 1024 == 0 {
            break;
        }
        size /= 1024;
    }
    println!("init: {:x}, size {}{}B", kern_end, size, scale);
    unsafe {
        (*PAGE_ALLOCATOR.inner.get()).write(PageAllocator {
            head: kern_end as *mut u8,
            end: ram_end as *mut u8,
            free_list: core::ptr::null_mut(),
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

pub fn free_frame(frame: usize) {
    unsafe {
        (*PAGE_ALLOCATOR.inner.get())
            .assume_init_mut()
            .free_frame(frame);
    }
}
