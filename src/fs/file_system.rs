use crate::{
    alloc::Box,
    error::Result,
    fs::{Inode, Path},
};

pub trait FileSystem {
    fn path_to_inode(&self, path: &Path) -> Result<Box<Inode>>;
}
