use core::{
    alloc::Layout,
    cmp::Ordering,
    num::NonZero,
    ops::{Add, AddAssign, Sub},
    ptr::NonNull,
};

use static_assertions::const_assert;

use crate::num::AlignUp;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
#[cfg_attr(test, derive(Debug))]
pub(super) struct BlockSize(NonZero<usize>);

#[derive(PartialEq, Eq, Copy, Clone)]
pub(super) struct BlockLayout {
    size: BlockSize,
    align: BlockSize,
}

#[cfg_attr(test, derive(Debug))]
pub(super) struct Block {
    pub(super) next: Option<NonNull<Block>>,
    pub(super) size: BlockSize,
}

struct AlignmentReduction {
    total: BlockSize,
    excess: Option<BlockSize>,
}

const_assert!(size_of::<Block>() == 8);

pub(super) enum SizeFit {
    Misfit,
    Exact,
    Excess,
}

impl Block {
    pub(super) fn init_at(ptr: NonNull<u8>, size: BlockSize) -> NonNull<Self> {
        let mut ptr = ptr.cast::<Block>();
        unsafe {
            let block = ptr.as_mut();
            block.size = size;
            block.next = None;
        };
        ptr
    }

    pub(super) fn check_fit(&self, layout: BlockLayout) -> SizeFit {
        let offset =
            ((self as *const Block) as *const u8).align_offset(layout.align().byte_count());
        let size_after_align = match self.size.byte_count().checked_sub(offset) {
            Some(size) => size,
            None => return SizeFit::Misfit,
        };
        match size_after_align.cmp(&layout.size().byte_count()) {
            Ordering::Less => SizeFit::Misfit,
            Ordering::Equal if offset == 0 => SizeFit::Exact,
            _ => SizeFit::Excess,
        }
    }

    pub(super) fn end_ptr(&mut self) -> NonNull<Block> {
        let ptr = unsafe { (self as *mut Block).add(self.size.block_count()) };
        NonNull::new(ptr).unwrap()
    }

    pub(super) fn merge(&mut self, next: NonNull<Block>) {
        let next = unsafe { next.as_ref() };
        self.size += next.size;
        self.next = next.next;
    }

    pub(super) fn take_next(&mut self, other: &mut Self) {
        self.next = other.next.take()
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
    pub(super) fn extract_block(
        block: *mut Block,
        layout: BlockLayout,
    ) -> Option<(*mut u8, Option<NonNull<Block>>)> {
        unsafe {
            let end_of_block = block.add((*block).size.block_count());
            let AlignmentReduction { total, excess } =
                Self::calc_reduction_for_layout(block as *mut u8, layout.size(), layout.align());
            if total > (*block).size {
                return None;
            }
            let allocated = end_of_block.sub(total.block_count());
            (*block).size = ((*block).size - total).unwrap();
            let excess = excess.and_then(|excess_size| {
                let mut excess = NonNull::new(end_of_block.sub(excess_size.block_count()))?;
                excess.as_mut().size = excess_size;
                Some(excess)
            });
            Some((allocated as *mut u8, excess))
        }
    }

    pub(super) fn size(&self) -> BlockSize {
        self.size
    }
}

/// Keep allocation sizes aligned to multiple of `size_of::<Block>()`, and non-zero.
impl BlockSize {
    pub(super) fn from(byte_size: usize) -> Option<BlockSize> {
        Some(BlockSize(NonZero::new(
            byte_size.align_up(size_of::<Block>()),
        )?))
    }

    pub(super) fn block_count(&self) -> usize {
        self.0.get() / size_of::<Block>()
    }

    pub(super) fn byte_count(&self) -> usize {
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

impl BlockLayout {
    pub(super) fn from(layout: Layout) -> BlockLayout {
        Self {
            size: BlockSize::from(layout.size()).unwrap(),
            align: BlockSize::from(layout.align()).unwrap(),
        }
    }

    pub(super) fn size(&self) -> BlockSize {
        self.size
    }

    pub(super) fn align(&self) -> BlockSize {
        self.align
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_reduction_aligned_without_excess() {
        let reduction = Block::calc_reduction_for_layout(
            0x1000 as *const u8,
            BlockSize::from(8).unwrap(),
            BlockSize::from(8).unwrap(),
        );
        assert_eq!(reduction.total.byte_count(), 8);
        assert_eq!(reduction.excess, None);
    }

    #[test_case]
    fn test_reduction_unaligned_without_excess() {
        let reduction = Block::calc_reduction_for_layout(
            0x1008 as *const u8,
            BlockSize::from(8).unwrap(),
            BlockSize::from(16).unwrap(),
        );
        assert_eq!(reduction.total.byte_count(), 8);
        assert_eq!(reduction.excess, None);
    }

    #[test_case]
    fn test_reduction_unaligned_without_excess2() {
        let reduction = Block::calc_reduction_for_layout(
            0x1008 as *const u8,
            BlockSize::from(16).unwrap(),
            BlockSize::from(8).unwrap(),
        );
        assert_eq!(reduction.total.byte_count(), 16);
        assert_eq!(reduction.excess, None);
    }

    #[test_case]
    fn test_reduction_aligned_with_excess() {
        let reduction = Block::calc_reduction_for_layout(
            0x1008 as *const u8,
            BlockSize::from(16).unwrap(),
            BlockSize::from(32).unwrap(),
        );
        assert_eq!(reduction.total.byte_count(), 40);
        assert_eq!(reduction.excess.unwrap().byte_count(), 24);
    }

    #[test_case]
    fn test_reduction_unaligned_with_excess() {
        let reduction = Block::calc_reduction_for_layout(
            0x1008 as *const u8,
            BlockSize::from(32).unwrap(),
            BlockSize::from(16).unwrap(),
        );
        assert_eq!(reduction.total.byte_count(), 40);
        assert_eq!(reduction.excess.unwrap().byte_count(), 8);
    }

    #[test_case]
    fn test_reduction_unaligned_with_excess() {
        let reduction = Block::calc_reduction_for_layout(
            0x1008 as *const u8,
            BlockSize::from(32).unwrap(),
            BlockSize::from(16).unwrap(),
        );
        assert_eq!(reduction.total.byte_count(), 40);
        assert_eq!(reduction.excess.unwrap().byte_count(), 8);
    }

    #[test_case]
    fn test_reduction_unaligned_with_excess() {
        let reduction = Block::calc_reduction_for_layout(
            0x1008 as *const u8,
            BlockSize::from(32).unwrap(),
            BlockSize::from(16).unwrap(),
        );
        assert_eq!(reduction.total.byte_count(), 40);
        assert_eq!(reduction.excess.unwrap().byte_count(), 8);
    }
}
