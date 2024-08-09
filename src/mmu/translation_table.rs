use crate::mmu::addr_parts::AddrParts;
use crate::mmu::entry::{Entry, EntryKind};

const L1_ENTRY_COUNT: usize = 4096;

pub struct TranslationTable<'a> {
    table: &'a mut [Entry],
}

impl<'a> TranslationTable<'a> {
    pub fn from_base(base: usize) -> Self {
        Self {
            table: unsafe { core::slice::from_raw_parts_mut(base as *mut Entry, L1_ENTRY_COUNT) },
        }
    }

    pub fn virt_to_phys(self, virt: usize) -> Option<usize> {
        let parts = AddrParts::from(virt);
        let entry = &self.table[parts.l1_index];
        match entry.get_type() {
            EntryKind::Unmapped => None,
            EntryKind::Section(section_base) => Some(section_base + parts.section_offset()),
            EntryKind::SeconLevelTable(l2_table) => {
                let l2_entry = &l2_table[parts.l2_index];
                l2_entry.get_phys().map(|addr| addr + parts.page_offset)
            }
            _ => panic!("Unsupported entry type"),
        }
    }
}
