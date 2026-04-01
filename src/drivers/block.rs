use crate::error::Result;

pub const SECTOR_SIZE: usize = 512;

pub trait Device {
    fn read(&self, buf: &mut [u8; SECTOR_SIZE], sector: usize) -> Result<()>;

    #[allow(unused)]
    fn write(&self, buf: &[u8; SECTOR_SIZE], sector: usize) -> Result<()>;

    fn ack_interrupt(&self);
}
