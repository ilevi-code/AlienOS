// extern crate alloc;

use crate::console::println;
use crate::num::Align;
use crate::phys::Phys;
use crate::spinlock::SpinLock;
use core::alloc::{GlobalAlloc, Layout, LayoutError};
use core::cell::UnsafeCell;
use core::cmp::{max, Ordering};
use core::mem::size_of;
use core::ops::{Add, Sub, SubAssign};

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

impl Sub for BlockSize {
    type Output = BlockSize;

    fn sub(self, rhs: Self) -> Self::Output {
        BlockSize { 0: self.0 - rhs.0 }
    }
}

impl Add for BlockSize {
    type Output = BlockSize;

    fn add(self, rhs: Self) -> Self::Output {
        BlockSize { 0: self.0 + rhs.0 }
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
            size: BlockSize::from(layout.size()),
            align: BlockSize::from(layout.align()),
        })
    }

    fn size(&self) -> BlockSize {
        self.size
    }

    fn align(&self) -> BlockSize {
        self.align
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
}

struct KernAlloctor {
    curr: *mut Block,
    end: *mut Block,
    free_list: Option<*mut Block>,
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
pub(crate) enum AllocError {
    #[error("{0}")]
    LayoutError(#[from] LayoutError),
    #[error("out of memory")]
    OutOfMem,
}

impl KernAlloctor {
    pub fn alloc(&mut self, layout: Layout) -> Result<Phys<u8>, AllocError> {
        let layout = BlockLayout::from(layout)?;
        let ptr = match self.look_for_freed_block(layout) {
            Some(block) => block,
            None => self.do_alloc(layout)?,
        };
        Ok(Phys::<u8>::from(ptr))
    }

    /// Looks for a block that is at big enough.
    /// If the block found is exactly the wanted size, it is popped from the free list.
    fn look_for_freed_block(&mut self, layout: BlockLayout) -> Option<*mut u8> {
        let mut iter = self.free_list;
        let mut prev: Option<&mut Block> = None;
        while let Some(current_ptr) = iter {
            let current = unsafe { &mut *current_ptr };
            match current.check_fit(layout) {
                SizeFit::Misfit => (),
                SizeFit::Excess => return Some(unsafe { self.extract_block(current_ptr, layout) }),
                SizeFit::Exact => {
                    match prev {
                        Some(prev) => prev.next = current.next.take(),
                        None => (),
                    };
                    return Some(current_ptr as *mut u8);
                }
            }
            iter = current.next;
            prev = Some(current);
        }
        None
    }

    fn calc_reduction_for_layout(
        ptr: *const u8,
        size: BlockSize,
        align: BlockSize,
    ) -> AlignmentReduction {
        let reduce_for_alignment = align - BlockSize(ptr.align_offset(align.byte_count()));
        match reduce_for_alignment.cmp(&size) {
            Ordering::Less => {
                // in case reduce_for_alignment is smaller than needed, we want to reduce the
                // bigger of either `size` or `align`.
                // If size is bigger - alignment is enforced.
                // If align is bigger - total size is guaranteed to be enough.
                // alignments.
                let total = reduce_for_alignment + max(size, align);
                AlignmentReduction {
                    total,
                    excess: Some(total - size),
                }
            }
            Ordering::Equal => AlignmentReduction {
                total: reduce_for_alignment,
                excess: None,
            },
            Ordering::Greater => AlignmentReduction {
                total: reduce_for_alignment,
                excess: Some(reduce_for_alignment - size),
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
        (*block).size -= total;
        if let Some(excess_size) = excess {
            let excess_ptr = end_of_block.sub(excess_size.block_count());
            self.free(excess_ptr as *mut u8, excess_size);
        }
        allocated as *mut u8
    }

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
            // SAFETY: `self.curr` is always aligned to `align_of::<Block>`, as well as `BlockSize`
            let size = unsafe { BlockSize::from_unchecked(bytes) };

            self.free(self.curr as *mut u8, size);
        }
    }

    fn bump_curr(&mut self, size: BlockSize) -> Result<(), AllocError> {
        if isize::try_from(size.byte_count()).is_err() {
            return Err(AllocError::OutOfMem);
        }
        let new_curr = self.curr.wrapping_add(size.block_count());
        // If wrapped
        // TODO,
        if (self.end..self.curr).contains(&new_curr) {
            return Err(AllocError::OutOfMem);
        }
        self.curr = new_curr;
        Ok(())
    }

    fn free(&mut self, _ptr: *mut u8, _size: BlockSize) {
        unimplemented!();
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
            Ok(ptr) => &mut *crate::memory_model::phys_to_virt::<u8>(&ptr),
            _ => core::ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.0
            .lock()
            .as_mut()
            // Also, this indicates free before alloc, since alloc should have paniced first
            .expect("Heap should be initilized before free")
            .free(
                ptr,
                BlockLayout::from(layout)
                    // The layout should be same as in alloc, if conversion have already succeeded once.
                    .expect("Free block layout creation should have succeeded")
                    .size(),
            )
    }
}

static ALLOCATOR: GlobalKernAllocator = GlobalKernAllocator {
    0: SpinLock::new(None),
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
    *ALLOCATOR.0.lock() = Some(KernAlloctor {
        curr: kern_end as *mut Block,
        end: ram_end as *mut Block,
        free_list: None,
    });
}

pub fn alloc<T>() -> Result<Phys<T>, AllocError> {
    let phys = ALLOCATOR
        .0
        .lock()
        .as_mut()
        .expect("Heap should be initlized before alloc")
        .alloc(Layout::new::<T>())?
        .cast::<T>();
    Ok(phys)
}
