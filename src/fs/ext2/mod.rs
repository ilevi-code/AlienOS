use crate::{
    alloc::{Arc, Box},
    drivers::block::Device,
    error::Result,
    fs::{File, FileSystem},
};

pub struct Ext2 {
    dev: Arc<dyn Device>,
}

impl Ext2 {
    pub fn new(dev: Arc<dyn Device>) -> Result<Self> {
        Ok(Self { dev })
    }
}

impl FileSystem for Ext2 {
    fn open(self: Arc<Self>, path: &super::Path) -> Result<Box<dyn File>> {
        todo!()
    }
}
