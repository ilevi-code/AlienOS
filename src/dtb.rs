#[repr(C)]
pub struct DeviceTree {
    magic: u32,
    total_size: u32,
    off_dt_struct: u32,
    off_dt_strings: u32,
    off_mem_rsvmap: u32,
    version: u32,
    last_comp_version: u32,
    boot_cpuid_phys: u32,
    size_dt_strings: u32,
    size_dt_struct: u32,
}

impl DeviceTree {
    fn total_size(&self) -> u32 {
        return u32::from_be(self.total_size);
    }
}
