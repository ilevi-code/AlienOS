//! Upon entry:
//! Running in the third GB (0x80000_0000..0xc000_0000), where the First GB of physicall memory is
//! mapped into.
//! The mmu is using fifty-fifty split, so we only have to change ttbr1 between
//! context-switches.
use crate::phys::Phys;

// Memory given by qemu
const MEM_START: usize = 0x4000_0000;
const PHYS_MAP_START: usize = 0x8000_0000;
const PHYS_TO_VIRT: usize = PHYS_MAP_START - MEM_START;

// See `init_stack` in boot.ld
pub const BOOT_STACK_SIZE: usize = 0x1000;

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
