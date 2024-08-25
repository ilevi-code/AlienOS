use crate::error::{Error, Result};
use core::alloc::Layout;
use core::ptr::NonNull;

use super::block::{Block, BlockLayout, BlockSize, SizeFit};

pub(super) struct KernAlloctor {
    curr: NonNull<Block>,
    end: *mut Block,
    free_list: Option<NonNull<Block>>,
}

impl KernAlloctor {
    pub(super) fn new(start: *mut u8, end: *mut u8) -> Self {
        Self {
            curr: NonNull::new(start as *mut Block).unwrap(),
            end: end as *mut Block,
            free_list: None,
        }
    }
    pub(super) fn alloc(&mut self, layout: Layout) -> Result<*mut u8> {
        let layout = BlockLayout::from(layout);
        let ptr = match self.look_for_freed_block(layout) {
            Some(block) => block,
            None => self.do_alloc(layout)?,
        };
        Ok(ptr)
    }

    /// Looks for a block that is at big enough.
    /// If the block found is exactly the wanted size, it is popped from the free list.
    fn look_for_freed_block(&mut self, layout: BlockLayout) -> Option<*mut u8> {
        let mut iter = self.free_list;
        let mut prev: Option<&mut Block> = None;
        while let Some(mut current_ptr) = iter {
            match unsafe { current_ptr.as_mut() }.check_fit(layout) {
                SizeFit::Misfit => (),
                SizeFit::Excess => {
                    let (allocated, excess) =
                        Block::extract_block(current_ptr.as_ptr(), layout).unwrap();
                    if let Some(excess) = excess {
                        self.free(excess);
                    }
                    return Some(allocated);
                }
                SizeFit::Exact => {
                    if let Some(prev) = prev {
                        prev.take_next(unsafe { current_ptr.as_mut() });
                    } else {
                        self.free_list = unsafe { current_ptr.as_mut() }.next;
                    }
                    return Some(current_ptr.as_ptr() as *mut u8);
                }
            }
            iter = unsafe { current_ptr.as_mut() }.next;
            prev = Some(unsafe { current_ptr.as_mut() });
        }
        None
    }

    /// Bumps the marker of currently used memory, freeing excess memory to enforce alignment.
    fn do_alloc(&mut self, layout: BlockLayout) -> Result<*mut u8> {
        self.force_alignment(layout.align());
        let ptr = self.curr;
        self.bump_curr(layout.size())?;
        Ok(ptr.cast::<u8>().as_ptr())
    }

    /// Force `self.curr` to be aligned to `size`.
    /// Any excess memory is free'd, and will be available for smaller allocation.
    fn force_alignment(&mut self, size: BlockSize) {
        if !self.curr.is_aligned_to(size.byte_count()) {
            let bytes = self.curr.align_offset(size.byte_count());
            // `self.curr` is always aligned to `align_of::<Block>`, as well as `BlockSize`
            let size = BlockSize::from(bytes).unwrap();

            self.free(Block::init_at(self.curr.cast::<u8>(), size));
            self.curr = unsafe { self.curr.add(size.block_count()) };
        }
    }

    fn bump_curr(&mut self, size: BlockSize) -> Result<()> {
        if isize::try_from(size.byte_count()).is_err() {
            return Err(Error::OutOfMem);
        }
        let new_curr = self.curr.as_ptr().wrapping_add(size.block_count());
        if !(self.curr.as_ptr()..=self.end).contains(&new_curr) {
            return Err(Error::OutOfMem);
        }
        self.curr = NonNull::new(new_curr).unwrap();
        Ok(())
    }

    fn free(&mut self, freed: NonNull<Block>) {
        // When starting to iterate over a dummy, it is easy to insert before the current
        // list-head.
        let dummy = Block {
            next: self.free_list,
            size: BlockSize::from(1).unwrap(),
        };

        let mut current = NonNull::from(&dummy);
        while let Some(next) = unsafe { current.as_mut() }.next {
            if (current..next).contains(&(freed.cast::<Block>())) {
                break;
            }
            current = next;
        }
        Self::merge_adjacent_blocks(current, freed);
        // Update the list head, `ptr` might inserted before the current head.
        self.free_list = dummy.next;
    }

    fn merge_adjacent_blocks(mut before: NonNull<Block>, mut freed: NonNull<Block>) {
        let freed = unsafe { freed.as_mut() };
        let before = unsafe { before.as_mut() };
        if let Some(next) = before.next {
            if freed.end_ptr() == next {
                freed.merge(next);
            } else {
                freed.next = Some(next);
            }
        }
        if before.end_ptr() == freed.into() {
            before.merge(freed.into());
        } else {
            before.next = Some(freed.into());
        }
    }

    pub(super) fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        self.free(Block::init_at(
            NonNull::<u8>::new(ptr).unwrap(),
            BlockLayout::from(layout).size(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::error::Error;

    use super::*;
    use core::hint::black_box;

    #[repr(align(1024))]
    struct AlignedBuffer([u8; 1024]);

    impl AlignedBuffer {
        fn as_allocator(&mut self) -> KernAlloctor {
            KernAlloctor::new(self.0.as_mut_ptr(), unsafe {
                self.0.as_mut_ptr().add(self.0.len())
            })
        }

        fn ptr_to_block(&mut self, block_idx: usize) -> *mut u8 {
            unsafe { self.0.as_mut_ptr().add(size_of::<Block>() * block_idx) }
        }
    }

    #[test_case]
    fn test_allocation() {
        let mut buf = AlignedBuffer([0; 1024]);
        let mut allocator = buf.as_allocator();
        assert_eq!(
            allocator.alloc(Layout::new::<u32>()).unwrap(),
            buf.ptr_to_block(0)
        );
    }

    #[test_case]
    fn test_two_allocation() {
        let mut buf = AlignedBuffer([0; 1024]);
        let mut allocator = buf.as_allocator();
        black_box(allocator.alloc(Layout::new::<u32>()).unwrap());
        assert_eq!(
            allocator.alloc(Layout::new::<u32>()).unwrap(),
            buf.ptr_to_block(1)
        );
    }

    #[test_case]
    fn test_alloc_free() {
        let mut buf = AlignedBuffer([0; 1024]);
        let mut allocator = buf.as_allocator();
        let layout = Layout::new::<u32>();
        let ptr = allocator.alloc(layout).unwrap();
        allocator.dealloc(ptr, layout);
        let free = unsafe { &mut *allocator.free_list.unwrap().as_mut() };
        assert_eq!(free as *mut Block, buf.ptr_to_block(0) as *mut Block);
        assert_eq!(free.size, BlockSize::from(size_of::<Block>()).unwrap());
    }

    #[test_case]
    fn test_double_alloc_free_unordered() {
        let mut buf = AlignedBuffer([0; 1024]);
        let mut allocator = buf.as_allocator();
        let layout = Layout::new::<u32>();
        let ptr1 = allocator.alloc(layout).unwrap();
        let ptr2 = allocator.alloc(layout).unwrap();
        allocator.dealloc(ptr2, layout);
        allocator.dealloc(ptr1, layout);
        let free = unsafe { &mut *allocator.free_list.unwrap().as_mut() };
        assert_eq!(free as *mut Block, buf.ptr_to_block(0) as *mut Block);
        assert_eq!(free.size, BlockSize::from(size_of::<Block>() * 2).unwrap());
    }

    #[test_case]
    fn test_double_alloc_free_ordered() {
        let mut buf = AlignedBuffer([0; 1024]);
        let mut allocator = buf.as_allocator();
        let layout = Layout::new::<u32>();
        let ptr1 = allocator.alloc(layout).unwrap();
        let ptr2 = allocator.alloc(layout).unwrap();
        allocator.dealloc(ptr1, layout);
        allocator.dealloc(ptr2, layout);
        let free = unsafe { &mut *allocator.free_list.unwrap().as_mut() };
        assert_eq!(free as *mut Block, buf.ptr_to_block(0) as *mut Block);
        assert_eq!(free.size, BlockSize::from(size_of::<Block>() * 2).unwrap());
    }

    #[test_case]
    fn test_alloc_needs_frees_excess() {
        let mut buf = AlignedBuffer([0; 1024]);
        let mut allocator = buf.as_allocator();

        let l1 = Layout::new::<u32>();
        let p1 = allocator.alloc(l1).unwrap();
        black_box(p1);

        let l2 = Layout::from_size_align(8, 16).unwrap();
        let p2 = allocator.alloc(l2).unwrap();
        assert_eq!(p2, buf.ptr_to_block(2).into());
        assert_eq!(
            allocator.free_list.unwrap().as_ptr(),
            buf.ptr_to_block(1) as *mut Block
        );
    }

    #[test_case]
    fn test_free_before_head_with_gap() {
        let mut buf = AlignedBuffer([0; 1024]);
        let mut allocator = buf.as_allocator();

        let l = Layout::new::<u32>();
        let p1 = allocator.alloc(l).unwrap();

        // Force a gap between p1 and p3
        let p2 = allocator.alloc(l).unwrap();
        black_box(p2);

        let p3 = allocator.alloc(l).unwrap();

        allocator.dealloc(p3, l);
        allocator.dealloc(p1, l);
        assert_eq!(
            allocator.free_list.unwrap().as_ptr(),
            buf.ptr_to_block(0) as *mut Block
        );
        assert_eq!(
            unsafe { &mut *allocator.free_list.unwrap().as_mut() }
                .next
                .unwrap()
                .as_ptr(),
            buf.ptr_to_block(2) as *mut Block
        );
    }

    #[test_case]
    fn test_alloc_all() {
        let mut buf = AlignedBuffer([0; 1024]);
        let mut allocator = buf.as_allocator();
        let l = Layout::new::<[u8; 512]>();
        let p1 = allocator.alloc(l).unwrap();
        let p2 = allocator.alloc(l).unwrap();
        assert_eq!(p1, buf.ptr_to_block(0));
        assert_eq!(p2, buf.ptr_to_block(512 / size_of::<Block>()));
    }

    #[test_case]
    fn test_out_of_mem() {
        let mut buf = AlignedBuffer([0; 1024]);
        let mut allocator = buf.as_allocator();
        let l = Layout::new::<[u8; 512]>();
        let p1 = allocator.alloc(l).unwrap();
        let p2 = allocator.alloc(l).unwrap();
        black_box(p1);
        black_box(p2);
        assert_eq!(
            allocator.alloc(Layout::new::<u8>()).unwrap_err(),
            Error::OutOfMem
        );
    }

    #[test_case]
    fn test_too_big_alloc() {
        let mut buf = AlignedBuffer([0; 1024]);
        let mut allocator = buf.as_allocator();
        let l = Layout::new::<[u8; 1025]>();
        assert_eq!(allocator.alloc(l).unwrap_err(), Error::OutOfMem);
    }

    #[test_case]
    fn test_alloc_removes_from_free_list() {
        let mut buf = AlignedBuffer([0; 1024]);
        let mut allocator = buf.as_allocator();
        let l = Layout::new::<Block>();
        let p1 = allocator.alloc(l).unwrap();
        allocator.dealloc(p1, l);
        let p2 = allocator.alloc(l).unwrap();
        let p3 = allocator.alloc(l).unwrap();
        assert_ne!(p2, p3);
    }
}
