use crate::drivers::block::SECTOR_SIZE;

pub const SUPERBLOCK_OFFSET: usize = 1024;
pub const SUPERBLOCK_SECTOR: usize = SUPERBLOCK_OFFSET / SECTOR_SIZE;

pub const INODE_SIZE_128B: u16 = 128;

pub const INODE_TABLE_START_BLOCK: u32 = 5;

pub const ROOT_INODE: usize = 2;

pub const EXT2_FT_DIR: u8 = 2;

pub const INDIRECT_BLOCK_INDEX: u32 = 12;
pub const DIRECT_BLOCK_COUNT: usize = INDIRECT_BLOCK_INDEX as usize;
