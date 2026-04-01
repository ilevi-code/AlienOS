use core::ops::{Index, IndexMut};
use core::slice::SliceIndex;

use crate::mmu::l2entry::L2Entry;
use crate::mmu::PagePerm;
use crate::phys::{Phys, PhysMut};

#[allow(unused)]
const L1_ENTRY_COUNT: usize = 4096;
const L2_ENTRY_COUNT: usize = 256;

pub(super) struct Entry {
    value: usize,
}

#[repr(align(1024))]
pub(super) struct SeconLevelTable([L2Entry; L2_ENTRY_COUNT]);

impl<I: SliceIndex<[L2Entry]>> Index<I> for SeconLevelTable {
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        Index::index(&self.0, index)
    }
}

impl<I: SliceIndex<[L2Entry]>> IndexMut<I> for SeconLevelTable {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        IndexMut::index_mut(&mut self.0, index)
    }
}

pub(super) type Section = [u8; 1024 * 1024];

pub(super) enum EntryKind {
    Unmapped,
    SeconLevelTable(Phys<SeconLevelTable>),
    Section(#[allow(unused)] Phys<Section>),
    SuperSection,
}

pub(super) enum EntryKindMut {
    Unmapped,
    SeconLevelTable(PhysMut<SeconLevelTable>),
    Section(#[allow(unused)] PhysMut<Section>),
    SuperSection,
}

impl Entry {
    const SUPERSECTION_BIT: usize = 1 << 18;
    const SECTION_MASK: usize = 0xfff00000;
    const L2_TABLE_MASK: usize = 0xfffffc00;

    const IGNORED_ENTRY_MAGIC: usize = 0b00;
    const SECOND_LEVEL_TABLE_MAGIC: usize = 0b01;
    const SECTION_MAGIC: usize = 0b10;

    pub fn get_type(&self) -> EntryKind {
        match self.value & 0b11 {
            Self::IGNORED_ENTRY_MAGIC => EntryKind::Unmapped,
            Self::SECOND_LEVEL_TABLE_MAGIC => EntryKind::SeconLevelTable(self.as_l2_table()),
            Self::SECTION_MAGIC => {
                if self.is_supersection() {
                    EntryKind::SuperSection
                } else {
                    EntryKind::Section(self.as_section())
                }
            }
            _ => panic!("Unsupported entry type"),
        }
    }

    pub fn get_type_mut(&mut self) -> EntryKindMut {
        match self.value & 0b11 {
            Self::IGNORED_ENTRY_MAGIC => EntryKindMut::Unmapped,
            Self::SECOND_LEVEL_TABLE_MAGIC => EntryKindMut::SeconLevelTable(self.as_l2_table_mut()),
            Self::SECTION_MAGIC => {
                if self.is_supersection() {
                    EntryKindMut::SuperSection
                } else {
                    EntryKindMut::Section(self.as_section_mut())
                }
            }
            _ => panic!("Unsupported entry type"),
        }
    }

    fn as_l2_table(&self) -> Phys<SeconLevelTable> {
        (self.value & Self::L2_TABLE_MASK).into()
    }

    fn as_l2_table_mut(&mut self) -> PhysMut<SeconLevelTable> {
        (self.value & Self::L2_TABLE_MASK).into()
    }

    fn as_section(&self) -> Phys<Section> {
        (self.value & Self::SECTION_MASK).into()
    }

    fn as_section_mut(&self) -> PhysMut<Section> {
        (self.value & Self::SECTION_MASK).into()
    }

    pub fn set_section(&mut self, phys: usize, perm: PagePerm, domain: u8) {
        self.value = (phys & Self::SECTION_MASK)
            | (perm.translate() << 10)
            | ((domain as usize) << 5)
            | (Self::SECTION_MAGIC);
    }

    pub(super) fn set_l2_table(&mut self, phys: PhysMut<SeconLevelTable>, domain: u8) {
        self.value = (phys.addr() & Self::L2_TABLE_MASK)
            | ((domain as usize) << 5)
            | Self::SECOND_LEVEL_TABLE_MAGIC;
    }

    pub fn unmap(&mut self) {
        self.value = 0;
    }

    fn is_supersection(&self) -> bool {
        (self.value & Self::SUPERSECTION_BIT) == Self::SUPERSECTION_BIT
    }
}
