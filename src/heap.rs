use crate::console::println;
use crate::memory_model::virt_to_phys;
use crate::num::AlignUp;
use crate::phys::Phys;
use crate::spinlock::SpinLock;
use core::alloc::{GlobalAlloc, Layout, LayoutError};
use core::cmp::Ordering;
use core::mem::size_of;
use core::num::NonZero;
use core::ops::{Add, AddAssign, Sub};
use core::ptr::NonNull;
use static_assertions::const_assert;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
#[cfg_attr(test, derive(Debug))]
struct BlockSize(NonZero<usize>);

/// Keep allocation sizes aligned to multiple of `size_of::<Block>()`
impl BlockSize {
    fn from(byte_size: usize) -> Option<BlockSize> {
        Some(BlockSize(NonZero::new(
            byte_size.align_up(size_of::<Block>()),
        )?))
    }

    fn block_count(&self) -> usize {
        self.0.get() / size_of::<Block>()
    }

    fn byte_count(&self) -> usize {
        self.0.get()
    }
}

impl Sub for BlockSize {
    type Output = Option<BlockSize>;

    fn sub(self, rhs: Self) -> Self::Output {
        Some(BlockSize(NonZero::new(self.0.get() - rhs.0.get())?))
    }
}

impl Add for BlockSize {
    type Output = BlockSize;

    fn add(self, rhs: Self) -> Self::Output {
        BlockSize::from(self.0.get() + rhs.0.get()).unwrap()
    }
}

impl AddAssign for BlockSize {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = NonZero::new(self.0.get() + rhs.0.get()).unwrap()
    }
}

#[derive(PartialEq, Eq, Copy, Clone)]
struct BlockLayout {
    size: BlockSize,
    align: BlockSize,
}

impl BlockLayout {
    fn from(layout: Layout) -> Result<BlockLayout, LayoutError> {
        Ok(Self {
            size: BlockSize::from(layout.size()).unwrap(),
            align: BlockSize::from(layout.align()).unwrap(),
        })
    }

    fn size(&self) -> BlockSize {
        self.size
    }

    fn align(&self) -> BlockSize {
        self.align
    }
}

#[cfg_attr(test, derive(Debug))]
struct Block {
    next: Option<NonNull<Block>>,
    size: BlockSize,
}

enum SizeFit {
    Misfit,
    Exact,
    Excess,
}

impl Block {
    fn check_fit(&self, layout: BlockLayout) -> SizeFit {
        let offset =
            ((self as *const Block) as *const u8).align_offset(layout.align().byte_count());
        let size_after_align = self.size.byte_count() - offset;
        match size_after_align.cmp(&layout.size().byte_count()) {
            Ordering::Less => SizeFit::Misfit,
            Ordering::Equal if offset == 0 => SizeFit::Exact,
            _ => SizeFit::Excess,
        }
    }

    fn end_ptr(&mut self) -> NonNull<Block> {
        let ptr = unsafe { (self as *mut Block).add(self.size.block_count()) };
        NonNull::new(ptr).unwrap()
    }

    fn merge(&mut self, next: NonNull<Block>) {
        let next = unsafe { next.as_ref() };
        self.size += next.size;
        self.next = next.next;
    }
}

struct KernAlloctor {
    curr: *mut Block,
    end: *mut Block,
    free_list: Option<NonNull<Block>>,
}

struct AlignmentReduction {
    total: BlockSize,
    excess: Option<BlockSize>,
}

enum BlockMatch {
    Exact(*mut Block),
    Bigger(*mut Block),
    None,
}

use thiserror_no_std::Error;
#[derive(Error, Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub(crate) enum AllocError {
    #[error("{0}")]
    LayoutError(#[from] LayoutError),
    #[error("out of memory")]
    OutOfMem,
}

impl KernAlloctor {
    fn alloc(&mut self, layout: Layout) -> Result<*mut u8, AllocError> {
        let layout = BlockLayout::from(layout)?;
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
                    return Some(unsafe { self.extract_block(current_ptr.as_ptr(), layout) })
                }
                SizeFit::Exact => {
                    if let Some(prev) = prev {
                        prev.next = unsafe { current_ptr.as_mut() }.next.take()
                    }
                    return Some(current_ptr.as_ptr() as *mut u8);
                }
            }
            iter = unsafe { current_ptr.as_mut() }.next;
            prev = Some(unsafe { current_ptr.as_mut() });
        }
        None
    }

    fn calc_reduction_for_layout(
        ptr: *const u8,
        size: BlockSize,
        align: BlockSize,
    ) -> AlignmentReduction {
        let aligned_size = BlockSize::from(size.byte_count().align_up(align.byte_count())).unwrap();
        let reduce_for_alignment = match BlockSize::from(ptr.align_offset(align.byte_count()))
            .and_then(|offset| align - offset)
        {
            None => {
                return AlignmentReduction {
                    total: aligned_size,
                    excess: aligned_size - size,
                }
            }
            Some(r) => r,
        };
        match reduce_for_alignment.cmp(&size) {
            Ordering::Less => {
                let total = if size <= align {
                    // If align is bigger (or equal) - total size is guaranteed to be enough, and
                    // alignment is enforced.
                    align + reduce_for_alignment
                } else {
                    aligned_size + reduce_for_alignment
                };
                AlignmentReduction {
                    total,
                    excess: total - size,
                }
            }
            Ordering::Equal => AlignmentReduction {
                total: reduce_for_alignment,
                excess: None,
            },
            Ordering::Greater => AlignmentReduction {
                total: reduce_for_alignment,
                excess: reduce_for_alignment - size,
            },
        }
    }

    /// Extract a sub-block of `size` from the end of `block`.
    ///
    /// SAFETY: The memory pointed to by `block` must be bigger or equal than `Layout.size()`.
    unsafe fn extract_block(&mut self, block: *mut Block, layout: BlockLayout) -> *mut u8 {
        let end_of_block = block.add((*block).size.block_count());
        let AlignmentReduction { total, excess } =
            Self::calc_reduction_for_layout(block as *mut u8, layout.size(), layout.align());
        let allocated = end_of_block.sub(total.block_count());
        (*block).size = ((*block).size - total).unwrap();
        if let Some(excess_size) = excess {
            let excess_ptr = end_of_block.sub(excess_size.block_count());
            self.free(excess_ptr as *mut u8, excess_size);
        }
        allocated as *mut u8
    }

    /// Bumps the marker of currently used memory, freeing excess memory to enforce alignment.
    fn do_alloc(&mut self, layout: BlockLayout) -> Result<*mut u8, AllocError> {
        self.force_alignment(layout.align());
        let ptr = self.curr;
        self.bump_curr(layout.size())?;
        Ok(ptr as *mut u8)
    }

    /// Force `self.curr` to be aligned to `size`.
    /// Any excess memory is free'd, and will be available for smaller allocation.
    fn force_alignment(&mut self, size: BlockSize) {
        if !self.curr.is_aligned_to(size.byte_count()) {
            let bytes = self.curr.align_offset(size.byte_count());
            // `self.curr` is always aligned to `align_of::<Block>`, as well as `BlockSize`
            let size = BlockSize::from(bytes).unwrap();

            self.free(self.curr as *mut u8, size);
            self.curr = unsafe { self.curr.add(size.block_count()) };
        }
    }

    fn bump_curr(&mut self, size: BlockSize) -> Result<(), AllocError> {
        if isize::try_from(size.byte_count()).is_err() {
            return Err(AllocError::OutOfMem);
        }
        let new_curr = self.curr.wrapping_add(size.block_count());
        if !(self.curr..=self.end).contains(&new_curr) {
            return Err(AllocError::OutOfMem);
        }
        self.curr = new_curr;
        Ok(())
    }

    fn free(&mut self, ptr: *mut u8, size: BlockSize) {
        let freed = Self::create_freed_block(ptr, size);

        // When starting to iterate over a dummy, it is easy to insert before the current
        // list-head.
        let dummy = Block {
            next: self.free_list,
            size: BlockSize::from(1).unwrap(),
        };

        let mut current = NonNull::from(&dummy);
        while let Some(next) = unsafe { current.as_mut() }.next {
            if (current.as_ptr()..next.as_ptr()).contains(&(ptr as *mut Block)) {
                break;
            }
            current = next;
        }
        Self::merge_adjacent_blocks(current, freed);
        // Update the list head, `ptr` might inserted before the current head.
        self.free_list = dummy.next;
    }

    fn create_freed_block(ptr: *mut u8, size: BlockSize) -> NonNull<Block> {
        let ptr = ptr as *mut Block;
        let block = unsafe { &mut *ptr };
        block.size = size;
        block.next = None;
        NonNull::new(ptr).unwrap()
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

    pub fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        self.free(
            ptr,
            BlockLayout::from(layout)
                // The layout should be same as in alloc, if conversion have already succeeded once.
                .expect("Free block layout creation should have succeeded")
                .size(),
        )
    }
}

struct GlobalKernAllocator(SpinLock<Option<KernAlloctor>>);

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

static ALLOCATOR: GlobalKernAllocator = GlobalKernAllocator(SpinLock::new(None));

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
    *ALLOCATOR.0.lock() = Some(KernAlloctor {
        curr: kern_end as *mut Block,
        end: ram_end as *mut Block,
        free_list: None,
    });
}

pub fn alloc<T>() -> Result<Phys<T>, AllocError> {
    let virt = ALLOCATOR
        .0
        .lock()
        .as_mut()
        .expect("Heap should be initlized before alloc")
        .alloc(Layout::new::<T>())?
        .cast::<T>();
    Ok(virt_to_phys(virt))
}

const_assert!(size_of::<Block>() == 8);

#[cfg(test)]
mod tests {
    use super::*;
    use core::hint::black_box;

    #[test_case]
    fn test_reduction_aligned_without_excess() {
        let reduction = KernAlloctor::calc_reduction_for_layout(
            0x1000 as *const u8,
            BlockSize::from(8).unwrap(),
            BlockSize::from(8).unwrap(),
        );
        assert_eq!(reduction.total.byte_count(), 8);
        assert_eq!(reduction.excess, None);
    }

    #[test_case]
    fn test_reduction_unaligned_without_excess() {
        let reduction = KernAlloctor::calc_reduction_for_layout(
            0x1008 as *const u8,
            BlockSize::from(8).unwrap(),
            BlockSize::from(16).unwrap(),
        );
        assert_eq!(reduction.total.byte_count(), 8);
        assert_eq!(reduction.excess, None);
    }

    #[test_case]
    fn test_reduction_unaligned_without_excess2() {
        let reduction = KernAlloctor::calc_reduction_for_layout(
            0x1008 as *const u8,
            BlockSize::from(16).unwrap(),
            BlockSize::from(8).unwrap(),
        );
        assert_eq!(reduction.total.byte_count(), 16);
        assert_eq!(reduction.excess, None);
    }

    #[test_case]
    fn test_reduction_aligned_with_excess() {
        let reduction = KernAlloctor::calc_reduction_for_layout(
            0x1008 as *const u8,
            BlockSize::from(16).unwrap(),
            BlockSize::from(32).unwrap(),
        );
        assert_eq!(reduction.total.byte_count(), 40);
        assert_eq!(reduction.excess.unwrap().byte_count(), 24);
    }

    #[test_case]
    fn test_reduction_unaligned_with_excess() {
        let reduction = KernAlloctor::calc_reduction_for_layout(
            0x1008 as *const u8,
            BlockSize::from(32).unwrap(),
            BlockSize::from(16).unwrap(),
        );
        assert_eq!(reduction.total.byte_count(), 40);
        assert_eq!(reduction.excess.unwrap().byte_count(), 8);
    }

    #[test_case]
    fn test_reduction_unaligned_with_excess() {
        let reduction = KernAlloctor::calc_reduction_for_layout(
            0x1008 as *const u8,
            BlockSize::from(32).unwrap(),
            BlockSize::from(16).unwrap(),
        );
        assert_eq!(reduction.total.byte_count(), 40);
        assert_eq!(reduction.excess.unwrap().byte_count(), 8);
    }

    #[test_case]
    fn test_reduction_unaligned_with_excess() {
        let reduction = KernAlloctor::calc_reduction_for_layout(
            0x1008 as *const u8,
            BlockSize::from(32).unwrap(),
            BlockSize::from(16).unwrap(),
        );
        assert_eq!(reduction.total.byte_count(), 40);
        assert_eq!(reduction.excess.unwrap().byte_count(), 8);
    }

    #[repr(align(1024))]
    struct AlignedBuffer([u8; 1024]);

    impl AlignedBuffer {
        fn as_allocator(&mut self) -> KernAlloctor {
            KernAlloctor {
                curr: (self.0.as_ptr()) as *mut Block,
                end: unsafe { self.0.as_ptr().add(self.0.len()) } as *mut Block,
                free_list: None,
            }
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
            AllocError::OutOfMem
        );
    }

    #[test_case]
    fn test_too_big_alloc() {
        let mut buf = AlignedBuffer([0; 1024]);
        let mut allocator = buf.as_allocator();
        let l = Layout::new::<[u8; 1025]>();
        assert_eq!(allocator.alloc(l).unwrap_err(), AllocError::OutOfMem);
    }
}
