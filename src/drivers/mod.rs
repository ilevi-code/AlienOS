pub mod block;
pub mod virtio_blk;

use block::{Device, SECTOR_SIZE};
use virtio_blk::{block::Request, regs::VirtioRegs, virt_queue::VirtQueue, VirtioBlk};
