use crate::{
    alloc::Box,
    error::Error,
    fs::{FileSystem, Inode, Path},
};

pub struct NullFs {}

impl NullFs {
    pub fn new() -> Self {
        Self {}
    }
}

impl FileSystem for NullFs {
    fn path_to_inode(&self, _path: &Path) -> crate::error::Result<Box<Inode>> {
        Err(Error::Unsupproted)
    }
}
