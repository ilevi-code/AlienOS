use core::ptr::NonNull;

use crate::{
    alloc::{Arc, Box, Vec},
    drivers::block::{Device, SECTOR_SIZE},
    error::{Error, Result},
    fs::{
        ext2::{
            consts::{
                DIRECT_BLOCK_COUNT, EXT2_FT_DIR, INODE_SIZE_128B, INODE_TABLE_START_BLOCK,
                ROOT_INODE, SUPERBLOCK_SECTOR,
            },
            dir_entries::DirEntries,
            inode::Inode,
            revision::Revision,
            superblock::Superblock,
        },
        path::Components,
        File, FileSystem, Path,
    },
};
use static_assertions::const_assert;

pub struct Ext2 {
    dev: Arc<dyn Device>,
    superblock: Box<Superblock>,
}

impl Ext2 {
    pub fn new(dev: Arc<dyn Device>) -> Result<Self> {
        const_assert!(core::mem::size_of::<Superblock>() < SECTOR_SIZE);

        let mut superblock = Box::<Superblock>::new_uninit()?;
        let mut buf = [0u8; SECTOR_SIZE];
        dev.read(&mut buf, SUPERBLOCK_SECTOR)?;
        unsafe { superblock.write(NonNull::from_ref(&buf).cast::<Superblock>().read()) };
        let mut superblock = unsafe { Box::assume_init(superblock) };

        match Into::<Revision>::into(superblock.rev_level) {
            Revision::GoodOld => superblock.inode_size = INODE_SIZE_128B,
            Revision::Dynamic => {
                if superblock.inode_size as usize > SECTOR_SIZE {
                    return Err(Error::Unsupproted);
                }
            }
            Revision::Unknown => return Err(Error::Unsupproted),
        }

        Ok(Self { dev, superblock })
    }

    fn read_inode(&self, inode_number: usize) -> Result<Box<Inode>> {
        let mut inode = Box::<Inode>::new_uninit()?;

        let offset = self.offset_of_block(INODE_TABLE_START_BLOCK)
            + (inode_number - 1) * self.superblock.inode_size as usize;
        let sector = offset / SECTOR_SIZE;
        let offset_in_sector = offset % SECTOR_SIZE;

        let mut buf = [0u8; SECTOR_SIZE];
        self.dev.read(&mut buf, sector)?;
        // Safety:
        // We do not allow inode-sizes that are bigger than sector size, so
        // The offset_in_sector + inode_size will can't overflow the buffer.
        let inode_ptr = unsafe {
            NonNull::from_ref(&buf)
                .byte_offset(offset_in_sector as isize)
                .cast::<Inode>()
        };

        // Safety:
        // the pointer points to a part of buffer, and thus is valid for reads.
        inode.write(unsafe { inode_ptr.read() });
        // Safety:
        // `inode` is initialized.
        Ok(unsafe { Box::assume_init(inode) })
    }

    fn offset_of_block(&self, block_num: u32) -> usize {
        self.block_size() * block_num as usize
    }

    pub fn block_size(&self) -> usize {
        1024 << self.superblock.log_block_size
    }

    pub fn read_block(&self, block_num: u32) -> Result<Vec<u8>> {
        let mut data = Vec::<u8>::new();

        self.read_block_into(block_num, &mut data);

        Ok(data)
    }

    pub fn read_block_into(&self, block_num: u32, buf: &mut Vec<u8>) -> Result<()> {
        buf.resize(self.block_size(), 0)?;

        let start_sector = self.offset_of_block(block_num) / SECTOR_SIZE;
        let (chunks, _extra) = buf[..self.block_size()].as_chunks_mut::<SECTOR_SIZE>();
        for (sector, chunk) in (start_sector..).zip(chunks.iter_mut()) {
            self.dev.read(chunk, sector)?;
        }
        Ok(())
    }

    fn path_to_inode(&self, path: &Path) -> Result<crate::alloc::Box<Inode>> {
        // TODO support relative paths
        if path.bytes.first() != Some(&b'/') {
            return Err(Error::Unsupproted);
        }
        let mut inode = self.read_inode(ROOT_INODE)?;
        let mut is_dir = true;
        for component in Components::new(&path.bytes) {
            let mut next = None;
            for block_num in inode
                .block
                .iter()
                .take(core::cmp::min(DIRECT_BLOCK_COUNT, inode.blocks as usize))
            {
                if !is_dir {
                    return Err(Error::NotADir);
                }
                let block = self.read_block(*block_num)?;
                let entries = DirEntries::new(&block[..], self.block_size());
                let Some(entry) = entries.into_iter().find(|entry| entry.name == component) else {
                    continue;
                };
                is_dir = entry.file_type == EXT2_FT_DIR;
                next = Some(self.read_inode(entry.inode as usize)?);
                break;
            }
            match next {
                Some(next) => inode = next,
                None => return Err(Error::NoEntry),
            };
        }
        if is_dir {
            return Err(Error::IsADir);
        }
        Ok(inode)
    }
}

impl FileSystem for Ext2 {
    fn open(self: Arc<Self>, path: &Path) -> Result<Box<dyn File>> {
        let _inode = self.path_to_inode(path)?;
        todo!();
    }
}
