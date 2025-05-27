//! Upon entry:
//! Running the the forth GB (0xc0000_0000 to 0xffff_ffff)
//! Physical memory is mapped the the third GB (0x8000_0000 to 0xbfff_ffff)
//! The first to GB is also mapped (qemu devices and 1:1 for physical memory)
//!
//! To unmap everything we dont need in the forth GB.
//! It will be used for kernel allocated memory, and devices.
//!
//! Next, we update the location of devices, so we can unamp the first two GB (0x0 to 0x7fff_ffff)
//! They will be used for user process.
//! Next we turn on fifty-fifty MMU split, so we only have to change ttbr1 between
//! context-switches.
use crate::phys::Phys;

// Memory given by qemu
const MEM_START: usize = 0x4000_0000;
const PHYS_MAP_START: usize = 0xc000_0000;
const PHYS_TO_VIRT: usize = PHYS_MAP_START - MEM_START;

// 16MB left empty - to be mapped as devices
pub const DEVICE_VIRT: usize = 0xfe00_0000;

pub fn phys_to_virt<T>(phys: &Phys<T>) -> *mut T {
    (phys.addr() + PHYS_TO_VIRT) as *mut T
}

pub fn virt_to_phys<T>(phys: *mut T) -> Phys<T> {
    Phys::from(phys as usize - PHYS_TO_VIRT)
}

pub fn virt_to_phys_const<T>(phys: *const T) -> Phys<T> {
    Phys::from(phys as usize - PHYS_TO_VIRT)
}

pub fn phys_to_virt_mut<T>(phys: &Phys<T>) -> &'static mut T {
    let virt = phys_to_virt(phys);
    unsafe { &mut *virt }
}
