use core::{
    ptr::{self, NonNull},
    slice,
};

mod inode;
mod superblock;

use crate::{
    alloc::{Arc, Box, Vec},
    drivers::block::{Device, SECTOR_SIZE},
    error::{Error, Result},
    fs::{
        ext2::{inode::Inode, superblock::Superblock},
        path::Components,
        File, FileSystem,
    },
    sys::User,
};
use static_assertions::const_assert;

pub struct Ext2 {
    dev: Arc<dyn Device>,
    superblock: Box<Superblock>,
}

const SUPERBLOCK_OFFSET: usize = 1024;
const SUPERBLOCK_SECTOR: usize = SUPERBLOCK_OFFSET / SECTOR_SIZE;

const ROOT_INODE: usize = 2;

const INODE_TABLE_START_BLOCK: u32 = 5;

const INODE_SIZE_128B: u16 = 128;

const DIRECT_BLOCK_COUNT: usize = 12;

const EXT2_FT_DIR: u8 = 2;

enum Revision {
    GoodOld = 0,
    Dynamic = 1,
    Unknown,
}

impl Ext2 {
    pub fn new(dev: Arc<dyn Device>) -> Result<Self> {
        const_assert!(core::mem::size_of::<Superblock>() < SECTOR_SIZE);

        let mut superblock = Box::<Superblock>::new_uninit()?;
        let mut buf = [0u8; SECTOR_SIZE];
        dev.read(&mut buf, SUPERBLOCK_SECTOR)?;
        unsafe { superblock.write(NonNull::from_ref(&buf).cast::<Superblock>().read()) };
        let mut superblock = unsafe { Box::assume_init(superblock) };

        if superblock.rev_level == Revision::GoodOld as u32 {
            superblock.inode_size = INODE_SIZE_128B;
        } else if superblock.inode_size as usize > SECTOR_SIZE {
            return Err(Error::Unsupproted);
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

    fn block_size(&self) -> usize {
        1024 << self.superblock.log_block_size
    }

    fn read_block(&self, block_num: u32) -> Result<Vec<u8>> {
        let mut data = Vec::<u8>::new();

        data.resize(self.block_size(), 0)?;

        let start_sector = self.offset_of_block(block_num) / SECTOR_SIZE;
        for (sector, chunk) in
            (start_sector..).zip(data[..].as_chunks_mut::<SECTOR_SIZE>().0.iter_mut())
        {
            self.dev.read(chunk, sector)?;
        }
        Ok(data)
    }
}

pub struct DirEntries<'a> {
    dir_data: &'a [u8],
    pos: usize,
    block_size: usize,
}

impl<'a> DirEntries<'a> {
    pub fn new(dir_data: &'a [u8], block_size: usize) -> Self {
        Self {
            dir_data,
            pos: 0,
            block_size,
        }
    }
}

pub struct DirEntry<'a> {
    inode: u32,
    file_type: u8,
    name: &'a [u8],
}

#[repr(packed)]
struct RawDirEntry {
    inode: u32,
    record_length: u16,
    name_length: u8,
    file_type: u8,
}

impl<'a> Iterator for DirEntries<'a> {
    type Item = DirEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos == self.block_size {
            return None;
        }
        // Safety:
        // `self.pos` is contained within the block, and thus is a valid reference
        let entry_ptr = unsafe {
            ptr::from_ref(self.dir_data)
                .cast::<RawDirEntry>()
                .byte_add(self.pos)
        };
        // Safety:
        // TODO
        let raw_entry = unsafe { entry_ptr.as_ref_unchecked() };
        self.pos += raw_entry.record_length as usize;
        let entry = DirEntry {
            inode: raw_entry.inode,
            file_type: raw_entry.file_type,
            // Safety:
            // TODO
            name: unsafe {
                slice::from_raw_parts(
                    entry_ptr.add(1).cast::<u8>(),
                    raw_entry.name_length as usize,
                )
            },
        };
        Some(entry)
    }
}

impl Ext2 {
    fn path_to_inode(&self, path: &super::Path) -> Result<crate::alloc::Box<Inode>> {
        // TODO support relative paths
        if path.bytes.first() != Some(&b'/') {
            return Err(Error::Unsupproted);
        }
        let mut inode = self.read_inode(ROOT_INODE)?;
        let mut is_dir = true;
        for component in Components::new(&path.bytes) {
            let mut next = None;
            for block_num in inode.block.iter().take(DIRECT_BLOCK_COUNT) {
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

#[allow(unused)]
struct Ext2File {
    inode: Box<Inode>,
    fs: Arc<Ext2>,
}

impl File for Ext2File {
    fn read(&mut self, _buf: User<&[u8]>) -> core::result::Result<(), crate::sys::Errno> {
        todo!()
    }
}

impl FileSystem for Ext2 {
    fn open(self: Arc<Self>, path: &super::Path) -> Result<Box<dyn File>> {
        let inode = self.path_to_inode(path)?;
        let file: Box<dyn File> = Box::new(Ext2File { fs: self, inode })?;
        Ok(file)
    }
}
