use crate::{
    alloc::{Arc, Box},
    error::Result,
    fs::{File, Path},
};

pub trait FileSystem {
    fn open(self: Arc<Self>, path: &Path) -> Result<Box<dyn File>>;
}
