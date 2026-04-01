use crate::{
    alloc::{Arc, Box, Vec},
    error::Result,
    fs::{
        ext2::{consts::INDIRECT_BLOCK_INDEX, ext2::Ext2, inode::Inode},
        File,
    },
    sys::{copy_to_user, User},
};

pub struct Ext2File {
    inode: Box<Inode>,
    fs: Arc<Ext2>,
    file_offset: usize,
    indirect_block: Option<Vec<u8>>,
}

enum BlockIndex {
    Direct(usize),
    Indirect(usize),
}

struct Offset {
    in_block: u16,
    block_index: BlockIndex,
}

impl Ext2File {
    pub fn new(fs: Arc<Ext2>, inode: Box<Inode>) -> Self {
        Self {
            fs,
            inode,
            file_offset: 0,
            indirect_block: None,
        }
    }
    fn split_file_offset(&self) -> Offset {
        let block_size = self.fs.block_size();
        let in_block = (self.file_offset % block_size) as u16;
        let blocks_per_indirect = (block_size / size_of::<u32>()) as u32;
        let block_index = self.file_offset / block_size;
        let block_index = if block_index < INDIRECT_BLOCK_INDEX as usize {
            BlockIndex::Direct(block_index)
        } else if block_index < (INDIRECT_BLOCK_INDEX + blocks_per_indirect) as usize {
            BlockIndex::Indirect(block_index - INDIRECT_BLOCK_INDEX as usize)
        } else {
            todo!("support doubly indirect block");
        };
        Offset {
            in_block,
            block_index,
        }
    }

    fn block_index_to_block_num(&mut self, block_index: BlockIndex) -> Result<u32> {
        match block_index {
            BlockIndex::Direct(direct_index) => Ok(self.inode.block[direct_index]),
            BlockIndex::Indirect(indirect_index) => {
                let indirects = match self.indirect_block.take() {
                    Some(block) => block,
                    None => self
                        .fs
                        .read_block(self.inode.block[INDIRECT_BLOCK_INDEX as usize])?,
                };
                let (indirects, _excess) = indirects[..].as_chunks::<4>();
                let Some(block_num) = indirects.get(indirect_index) else {
                    panic!("block index outside of indirect block");
                };
                Ok(u32::from_le_bytes(*block_num))
            }
        }
    }
}

impl File for Ext2File {
    fn read(&mut self, mut user_buf: &mut [User<u8>]) -> Result<()> {
        let size_left = self.inode.size as usize - self.file_offset;
        if user_buf.len() > size_left {
            user_buf = &mut user_buf[..size_left];
        }

        let mut block = Vec::new();
        while !user_buf.is_empty() {
            let offset = self.split_file_offset();
            let block_num = self.block_index_to_block_num(offset.block_index)?;

            self.fs.read_block_into(block_num, &mut block)?;
            let copy_length =
                core::cmp::min(block.len() - offset.in_block as usize, user_buf.len());
            copy_to_user(&mut user_buf[..copy_length], &block[..])?;

            user_buf = &mut user_buf[copy_length..];
            self.file_offset += copy_length;
        }
        Ok(())
    }
}
