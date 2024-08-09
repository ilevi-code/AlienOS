use crate::mmu::l2entry::L2Entry;
use crate::mmu::PagePerm;

pub(super) struct Entry {
    value: usize,
}

pub(super) enum EntryKind<'a> {
    Unmapped,
    SeconLevelTable(&'a mut [L2Entry]),
    Section(usize),
    SuperSection,
}

impl Entry {
    const SUPERSECTION_BIT: usize = 1 << 18;
    const SECTION_MASK: usize = 0xfff00000;
    const L2_TABLE_MASK: usize = 0xfffffc00;
    pub(super) const L2_ENTRY_COUNT: usize = 256;

    const IGNORED_ENTRY_MAGIC: usize = 0b00;
    const SECOND_LEVEL_TABLE_MAGIC: usize = 0b01;
    const SECTION_MAGIC: usize = 0b10;

    pub fn get_type(&self) -> EntryKind {
        match self.value & 0b11 {
            Self::IGNORED_ENTRY_MAGIC => EntryKind::Unmapped,
            Self::SECOND_LEVEL_TABLE_MAGIC => {
                EntryKind::SeconLevelTable(self.as_l2_table_mut().unwrap())
            }
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

    fn as_l2_table_mut(&self) -> Option<&mut [L2Entry]> {
        if self.value & 0b11 != Self::SECOND_LEVEL_TABLE_MAGIC {
            None
        } else {
            Some(unsafe {
                core::slice::from_raw_parts_mut(
                    (self.value as *const L2Entry).cast_mut(),
                    Self::L2_ENTRY_COUNT,
                )
            })
        }
    }

    fn as_section(&self) -> usize {
        self.value & Self::SECTION_MASK
    }

    fn set_section(&mut self, phys: usize, perm: PagePerm, domain: u8) {
        self.value = (phys & Self::SECTION_MASK)
            | (perm.translate() << 10)
            | ((domain as usize) << 5)
            | (Self::SECTION_MAGIC);
    }

    pub(super) fn set_l2_table(&mut self, phys: usize, domain: u8) {
        self.value = (phys & Self::L2_TABLE_MASK)
            | ((domain as usize) << 5)
            | Self::SECOND_LEVEL_TABLE_MAGIC;
    }

    fn unmap(&mut self) {
        self.value = 0;
    }

    fn is_supersection(&self) -> bool {
        (self.value & Self::SUPERSECTION_BIT) == Self::SUPERSECTION_BIT
    }
}
