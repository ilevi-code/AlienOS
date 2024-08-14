// extern crate alloc;

use crate::console::println;
use crate::num::Align;
use crate::phys::Phys;
use core::alloc::Layout;
use core::cell::UnsafeCell;
use core::cmp::{max, Ordering};
use core::mem::{size_of, MaybeUninit};
use core::ops::SubAssign;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
struct BlockSize(usize);

/// Keep allocation sizes aligned to multiple of `size_of::<Block>()`
impl BlockSize {
    fn from(byte_size: usize) -> BlockSize {
        BlockSize {
            // TODO ceil_div
            0: byte_size.align_up(size_of::<Block>()),
        }
    }

    unsafe fn from_unchecked(byte_size: usize) -> BlockSize {
        BlockSize { 0: byte_size }
    }

    fn block_count(&self) -> usize {
        self.0 / size_of::<Block>()
    }

    fn byte_count(&self) -> usize {
        self.0
    }
}

impl SubAssign for BlockSize {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0
    }
}

struct Block {
    next: Option<*mut Block>,
    size: BlockSize,
}

enum SizeFit {
    Misfit,
    Exact,
    Excess,
}

impl Block {
    fn fit(&self, size: BlockSize) {
        // TODO
    }
}

struct AllocatorImpl {
    curr: *mut Block,
    end: *mut Block,
    free_list: Option<*mut Block>,
}

enum BlockMatch {
    Exact(*mut Block),
    Bigger(*mut Block),
    None,
}

impl AllocatorImpl {
    pub fn alloc(&mut self, layout: Layout) -> Phys<u8> {
        let size = BlockSize::from(max(layout.size(), layout.align()));
        let ptr = match self.look_for_freed_block(size) {
            BlockMatch::Exact(block) => block as *mut u8,
            // SAFERY: `block` is guaranteed to be bigger
            BlockMatch::Bigger(block) => unsafe { AllocatorImpl::extract_block(block, size) },
            BlockMatch::None => todo!("alloc more"),
        };
        Phys::<u8>::from(ptr)
    }

    /// Looks for a block that is at big enough.
    /// If the block found is exactly the wanted size, it is popped from the free list.
    fn look_for_freed_block(&mut self, size: BlockSize) -> BlockMatch {
        let mut iter = self.free_list;
        let mut prev: Option<&mut Block> = None;
        while let Some(current) = iter {
            let current = unsafe { &mut *current };
            match current.size.cmp(&size) {
                Ordering::Less => (),
                Ordering::Greater => return BlockMatch::Bigger(current),
                Ordering::Equal => {
                    match prev {
                        Some(prev) => prev.next = current.next.take(),
                        None => (),
                    };
                    return BlockMatch::Exact(current);
                }
            }
            iter = current.next;
            prev = Some(current);
        }
        BlockMatch::None
    }

    /// Extract a sub-block of `size` from the end of `block`.
    ///
    /// SAFETY: The memory pointed to by `block` must be bigger than `size`.
    unsafe fn extract_block(block: *mut Block, size: BlockSize) -> *mut u8 {
        let extracted = block
            .add((*block).size.block_count())
            .sub(size.block_count());
        (*block).size -= size;
        extracted as *mut u8
    }

    fn do_alloc(&mut self, size: BlockSize) -> *mut u8 {
        self.force_alignment(size);
        let ptr = self.curr;
        self.bump_curr(size);
        ptr as *mut u8
    }

    /// Force `self.curr` to be aligned to `size`.
    /// Any excess memory is free'd, and will be available for smaller allocation.
    fn force_alignment(&mut self, size: BlockSize) {
        if !self.curr.is_aligned_to(size.byte_count()) {
            let bytes = self.curr.align_offset(size.byte_count());
            // SAFETY: `self.curr` is always aligned to `align_of::<Block>`, as well as `BlockSize`
            let size = unsafe { BlockSize::from_unchecked(bytes) };

            self.free(self.curr as *mut u8, size);
            self.bump_curr(size);
        }
    }

    fn bump_curr(&mut self, size: BlockSize) {
        unsafe {
            let new_curr = self.curr.add(size.block_count());
            assert!(isize::try_from(size.byte_count()).is_ok());
            assert!(new_curr > self.curr);
            self.curr = new_curr;
        }
    }

    fn free(&mut self, ptr: *mut u8, size: BlockSize) {
        unimplemented!();
    }
}

struct Allocator(UnsafeCell<Option<AllocatorImpl>>);

unsafe impl Sync for Allocator {}

impl Allocator {
    fn get(&self) -> &mut Option<AllocatorImpl> {
        unsafe { &mut *self.0.get() }
    }
}

static PAGE_ALLOCATOR: Allocator = Allocator {
    0: UnsafeCell::new(None),
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
    println!("kalloc init: {:x}, size {}{}B", kern_end, size, scale);
    *PAGE_ALLOCATOR.get() = Some(AllocatorImpl {
        curr: kern_end as *mut Block,
        end: ram_end as *mut Block,
        free_list: None,
    });
}

pub fn alloc_frame() -> usize {
    // PAGE_ALLOCATOR.get().unwrap().alloc_frame()
    0
}

pub fn free_frame(frame: usize) {}
