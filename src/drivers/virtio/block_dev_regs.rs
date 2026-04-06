use crate::volatile_reg_read;

#[repr(C, packed)]
struct Geometry {
    cylinders: u16,
    heads: u8,
    sectros: u8,
}

#[repr(C, packed)]
struct Topology {
    physical_block_exp: u8,
    alignment_offset: u8,
    min_io_size: u16,
    opt_io_size: u32,
}
#[repr(C, packed)]
pub(super) struct VirtioBlkConfig {
    capacity_low: u32,
    capacity_high: u32,
    size_max: u32,
    seg_max: u32,
    geometry: Geometry,
    blk_size: u32,
    toplogy: Topology,
    writeback: u8,
    _unused0: u8,
    num_queues: u16,
    max_discard_sectors: u32,
    max_discard_seg: u32,
    discard_sector_alignemtn: u32,
    max_write_zeroed_sectors: u32,
    max_write_zeroed_seg: u32,
    write_zeroes_may_unmap: u8,
    _unused1: [u8; 3],
    max_secure_erase_sectors: u32,
    max_secure_erase_seg: u32,
    secure_erase_sector_alignment: u32,
}

impl VirtioBlkConfig {
    volatile_reg_read!(capacity_low);
}
