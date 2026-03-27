use crate::{
    alloc::{Arc, Box},
    error::Error,
    fs::{File, FileSystem, Path},
};

pub struct NullFs {}

impl NullFs {
    pub fn new() -> Self {
        Self {}
    }
}

impl FileSystem for NullFs {
    fn open(self: Arc<Self>, _path: &Path) -> crate::error::Result<Box<dyn File>> {
        Err(Error::Unsupproted)
    }
}
