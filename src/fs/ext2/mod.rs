use crate::{alloc::Arc, drivers::block::Device, error::Result, fs::FileSystem, println};

pub struct Ext2 {
    dev: Arc<dyn Device>,
}

impl Ext2 {
    pub fn new(dev: Arc<dyn Device>) -> Result<Self> {
        Ok(Self { dev })
    }
}

impl FileSystem for Ext2 {
    fn path_to_inode(&self, path: &super::Path) -> Result<crate::alloc::Box<super::Inode>> {
        todo!()
    }
}
