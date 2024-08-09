use crate::mmu::PagePerm;
use core::mem::size_of;
use static_assertions::const_assert;

#[derive(PartialEq, Eq)]
pub(super) enum L2EntryType {
    Unmapped,
    Small,
    Large,
}

#[repr(C)]
pub(super) struct L2Entry {
    value: usize,
}

impl L2Entry {
    #[inline]
    pub fn set_phys(&mut self, phys: usize, entry_type: L2EntryType) {
        let mask = match entry_type {
            L2EntryType::Unmapped => return,
            L2EntryType::Small => 0xfffff000,
            L2EntryType::Large => 0xffff0000,
        };
        self.value = phys & mask;
    }

    #[inline]
    pub fn get_phys(&self) -> Option<usize> {
        let mask = match self.get_type() {
            L2EntryType::Unmapped => return None,
            L2EntryType::Small => 0xfffff000,
            L2EntryType::Large => 0xffff0000,
        };
        Some(self.value & mask)
    }

    #[inline]
    pub(super) fn get_type(&self) -> L2EntryType {
        match self.value & 0b11 {
            0 => L2EntryType::Unmapped,
            1 => L2EntryType::Small,
            _ => L2EntryType::Large,
        }
    }

    #[inline]
    pub(super) fn set_perm(&mut self, perm: PagePerm) {
        self.value |= perm.translate()
    }
}

const_assert!(size_of::<L2Entry>() == 4);
