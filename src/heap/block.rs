use core::{
    alloc::Layout,
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

#[derive(PartialEq, Eq)]
#[cfg_attr(test, derive(Debug))]
struct LogicalExtraction {
    prefix: Option<BlockSize>,
    suffix: Option<BlockSize>,
}

#[derive(PartialEq, Eq)]
#[cfg_attr(test, derive(Debug))]
pub(super) struct ExtractionReult {
    pub(super) extracted: NonNull<Block>,
    pub(super) new_next: Option<NonNull<Block>>,
}

const_assert!(size_of::<Block>() == 8);

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

    fn logical_extract(
        block: NonNull<Block>,
        block_size: BlockSize,
        layout: BlockLayout,
    ) -> Option<LogicalExtraction> {
        let offset = block.cast::<u8>().align_offset(layout.align().byte_count());
        let size_after_align = block_size.byte_count().checked_sub(offset)?;
        let prefix = BlockSize::from(block_size.byte_count() - size_after_align);
        let suffix = BlockSize::from(size_after_align.checked_sub(layout.size.byte_count())?);
        Some(LogicalExtraction { prefix, suffix })
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

    /// Extract a sub-block of `size` from the end of `block`.
    ///
    /// # Safety:
    /// `block` must be convertiable to reference
    pub(super) unsafe fn extract_block(
        mut block: NonNull<Block>,
        layout: BlockLayout,
    ) -> Option<ExtractionReult> {
        let LogicalExtraction { prefix, suffix };
        let Block { next, size };

        {
            // Safety:
            // The caller unsures `block` is convertiable to reference
            let block = unsafe { block.as_ref() };
            Block { next, size } = *block;
        }
        LogicalExtraction { prefix, suffix } = Block::logical_extract(block, size, layout)?;

        let result = match (prefix, suffix) {
            (None, None) => ExtractionReult {
                extracted: block,
                new_next: next,
            },
            (Some(prefix_size), None) => {
                // Safety:
                // The logical_extract unsures that `block + prefix` is within the current block
                let extracted = unsafe { block.add(prefix_size.block_count()) };
                // Safety:
                // The caller unsures `block` is convertiable to reference
                let this = unsafe { block.as_mut() };
                this.size = (this.size - layout.size()).unwrap();
                ExtractionReult {
                    extracted,
                    new_next: Some(block),
                }
            }
            (None, Some(suffix_size)) => {
                // Safety:
                // The logical_extract vlidates that layout.size() is smaller then the block size.
                let mut remaining_ptr = unsafe { block.byte_add(layout.size().byte_count()) };
                // Safety:
                // `remaining_ptr` is inside the allocatoin which is `block`, and the caller unsures
                // `block` is convertiable to reference.
                let remaining = unsafe { remaining_ptr.as_mut() };
                remaining.size = suffix_size;
                remaining.next = next;
                ExtractionReult {
                    extracted: block,
                    new_next: Some(remaining_ptr),
                }
            }
            (Some(prefix_size), Some(suffix_size)) => {
                // Safety:
                // The logical_extract unsures that `block + prefix` is within the current block
                let extracted = unsafe { block.add(prefix_size.block_count()) };
                // Safety:
                // The logical_extract unsures that `block + prefix + layout.size` is within the current block
                let mut suffix_ptr = unsafe { extracted.add(layout.size().block_count()) };
                // Safety:
                // `suffix_ptr` is inside the allocatoin which is `block`, and the caller unsures
                // `block` is convertiable to reference.
                let suffix = unsafe { suffix_ptr.as_mut() };
                suffix.size = suffix_size;
                suffix.next = next;
                // Safety:
                // The caller unsures `block` is convertiable to reference
                let prefix = unsafe { block.as_mut() };
                prefix.size = prefix_size;
                prefix.next = Some(suffix_ptr);
                ExtractionReult {
                    extracted,
                    new_next: Some(block),
                }
            }
        };
        Some(result)
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
    fn extraction_without_suffix_and_prefix() {
        assert_eq!(
            Block::logical_extract(
                NonNull::new(0x1000 as *mut Block).unwrap(),
                BlockSize::from(16).unwrap(),
                BlockLayout {
                    size: BlockSize::from(16).unwrap(),
                    align: BlockSize::from(16).unwrap(),
                }
            ),
            Some(LogicalExtraction {
                prefix: None,
                suffix: None
            })
        );
    }

    #[test_case]
    fn extraction_only_prefix() {
        assert_eq!(
            Block::logical_extract(
                NonNull::new(0x1008 as *mut Block).unwrap(),
                BlockSize::from(24).unwrap(),
                BlockLayout {
                    size: BlockSize::from(16).unwrap(),
                    align: BlockSize::from(16).unwrap(),
                }
            ),
            Some(LogicalExtraction {
                prefix: Some(BlockSize::from(8).unwrap()),
                suffix: None
            })
        );
    }

    #[test_case]
    fn extraction_only_suffix() {
        assert_eq!(
            Block::logical_extract(
                NonNull::new(0x1000 as *mut Block).unwrap(),
                BlockSize::from(24).unwrap(),
                BlockLayout {
                    size: BlockSize::from(16).unwrap(),
                    align: BlockSize::from(16).unwrap(),
                }
            ),
            Some(LogicalExtraction {
                prefix: None,
                suffix: Some(BlockSize::from(8).unwrap()),
            })
        );
    }

    #[test_case]
    fn extraction_with_prefix_and_suffix() {
        assert_eq!(
            Block::logical_extract(
                NonNull::new(0x1008 as *mut Block).unwrap(),
                BlockSize::from(40).unwrap(),
                BlockLayout {
                    size: BlockSize::from(16).unwrap(),
                    align: BlockSize::from(16).unwrap(),
                }
            ),
            Some(LogicalExtraction {
                prefix: Some(BlockSize::from(8).unwrap()),
                suffix: Some(BlockSize::from(16).unwrap())
            })
        );
    }

    #[test_case]
    fn cant_extract_bigger_block() {
        assert_eq!(
            Block::logical_extract(
                NonNull::new(0x1000 as *mut Block).unwrap(),
                BlockSize::from(32).unwrap(),
                BlockLayout {
                    size: BlockSize::from(64).unwrap(),
                    align: BlockSize::from(8).unwrap(),
                }
            ),
            None,
        );
    }

    #[test_case]
    fn cant_extract_block_after_alignement() {
        assert_eq!(
            Block::logical_extract(
                NonNull::new(0x1008 as *mut Block).unwrap(),
                BlockSize::from(32).unwrap(),
                BlockLayout {
                    size: BlockSize::from(32).unwrap(),
                    align: BlockSize::from(16).unwrap(),
                }
            ),
            None,
        );
    }

    #[repr(align(1024))]
    #[allow(unused)]
    struct AlignedBuffer([u8; 1024]);

    impl AlignedBuffer {
        fn create_mock_block(
            &mut self,
            size: usize,
            next: usize,
            aligned_to: usize,
            // unaligned_to: Option<usize>,
        ) -> NonNull<Block> {
            let start = NonNull::from_mut(self).cast::<u8>();
            let mut block_ptr = unsafe { start.add(aligned_to) }.cast::<Block>();
            let block = unsafe { block_ptr.as_mut() };
            block.size = BlockSize::from(size).unwrap();
            block.next = NonNull::new(next as *mut Block);
            block_ptr
        }
    }

    #[test_case]
    fn extract_exact_match() {
        let mut buf = AlignedBuffer([0; 1024]);
        let block = buf.create_mock_block(16, 0x2000, 8);
        assert_eq!(
            unsafe {
                Block::extract_block(
                    block,
                    BlockLayout::from(Layout::from_size_align(16, 8).unwrap()),
                )
            },
            Some(ExtractionReult {
                extracted: block,
                new_next: NonNull::<Block>::new(0x2000 as *mut Block)
            })
        );
    }

    #[test_case]
    fn extract_with_only_suffix() {
        let mut buf = AlignedBuffer([0; 1024]);
        let block = buf.create_mock_block(32, 0x2000, 8);
        assert_eq!(
            unsafe {
                Block::extract_block(
                    block,
                    BlockLayout::from(Layout::from_size_align(16, 8).unwrap()),
                )
            },
            Some(ExtractionReult {
                extracted: block,
                new_next: Some(unsafe { block.byte_offset(16) }),
            })
        );
    }

    #[test_case]
    fn extract_with_only_prefix() {
        let mut buf = AlignedBuffer([0; 1024]);
        let block = buf.create_mock_block(24, 0x2000, 8);
        assert_eq!(
            unsafe {
                Block::extract_block(
                    block,
                    BlockLayout::from(Layout::from_size_align(16, 16).unwrap()),
                )
            },
            Some(ExtractionReult {
                extracted: unsafe { block.byte_offset(8) },
                new_next: Some(block),
            })
        );
        assert_eq!(
            unsafe { block.as_ref() }.next,
            NonNull::new(0x2000 as *mut Block)
        );
    }

    #[test_case]
    fn extract_with_prefix_and_suffix() {
        let mut buf = AlignedBuffer([0; 1024]);
        let block = buf.create_mock_block(64, 0x2000, 8);
        assert_eq!(
            unsafe {
                Block::extract_block(
                    block,
                    BlockLayout::from(Layout::from_size_align(32, 16).unwrap()),
                )
            },
            Some(ExtractionReult {
                extracted: unsafe { block.byte_offset(8) },
                new_next: Some(block),
            })
        );
        assert_eq!(
            unsafe { block.as_ref() }.next,
            Some(unsafe { block.byte_offset(40) })
        );
    }
}
