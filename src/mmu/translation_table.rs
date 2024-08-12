use crate::console::println;
use crate::kalloc;
use crate::phys::Phys;
use crate::step_range::StepRange;
use core::mem::size_of;
use core::ops::Range;

use crate::console::println;
use crate::kalloc;
use crate::mmu::addr_parts::AddrParts;
use crate::mmu::entry::{Entry, EntryKind, SeconLevelTable, Section};
use crate::mmu::error::{MapError, Result};
use crate::mmu::l2entry::L2EntryType;
use crate::mmu::PagePerm;
use crate::num::Align;
use crate::phys::Phys;
use crate::step_range::StepRange;

pub const SMALL_PAGE_SIZE: usize = 4096;
const L1_ENTRY_COUNT: usize = 4096;

type L1Table = [Entry; L1_ENTRY_COUNT];

pub struct TranslationTable<'a> {
    table: &'a mut L1Table,
}

impl<'a> TranslationTable<'a> {
    // Given to us by qemu
    const MEM_START: usize = 0x4000_0000;
    // Start of memory controlled by ttbr0 in 1:1 split.
    const PHYS_MAP_START: usize = 0x8000_0000;
    // In the current memory model, 1GB of the physical memory is mapped to the third quarter of
    // memory
    const PHYS_TO_VIRT: usize = TranslationTable::PHYS_MAP_START - TranslationTable::MEM_START;

    pub fn from_base(base: usize) -> Self {
        Self {
            table: unsafe { &mut *(base as *mut L1Table) },
        }
    }

    fn phys_to_virt<T>(phys: &Phys<T>) -> &'a mut T {
        let ptr = (phys.addr() + TranslationTable::PHYS_TO_VIRT) as *mut T;
        unsafe { &mut *ptr }
    }

    pub fn new() -> Self {
        let frame = kalloc::alloc_frame();
        let table_phys = Phys::<L1Table>::from(frame);
        Self {
            table: TranslationTable::phys_to_virt(&table_phys),
        }
        // Self {  }
    }

    pub fn map_sections(
        &mut self,
        virt: usize,
        phys: usize,
        section_count: usize,
        perm: PagePerm,
    ) -> Result<()> {
        const SECTION_SIZE: usize = size_of::<Section>();
        let virt_range = StepRange::new(virt, virt + (section_count * SECTION_SIZE), SECTION_SIZE);
        let phys_range = StepRange::new(phys, phys + (section_count * SECTION_SIZE), SECTION_SIZE);

        for (virt, phys) in virt_range.zip(phys_range) {
            let parts = AddrParts::from(virt);
            let entry = self.get_l1(parts.l1_index);

            match entry.get_type() {
                EntryKind::Unmapped => (),
                _ => return Err(MapError::Remap),
            };

            entry.set_section(phys, perm, 0);
        }
        Ok(())
    }

    pub fn map(&mut self, virt: usize, phys: usize, len: usize, perm: PagePerm) -> Result<()> {
        let virt_range = StepRange::new(virt, virt + len, SMALL_PAGE_SIZE);
        let phys_range = StepRange::new(phys, phys + len, SMALL_PAGE_SIZE);

        println!(
            "mapping virt 0x{:x}[0..0x{:x}] to phys 0x{:x}",
            virt, len, phys
        );

        for (virt, phys) in virt_range.zip(phys_range) {
            let addr = AddrParts::from(virt);
            self.map_once(&addr, phys, perm)?;
        }
        Ok(())
    }

    fn map_once(&mut self, addr: &AddrParts, phys: usize, perm: PagePerm) -> Result<()> {
        let entry = self.get_l1(addr.l1_index);

        let l2_table = match entry.get_type() {
            EntryKind::SeconLevelTable(l2_table) => TranslationTable::phys_to_virt(&l2_table),
            EntryKind::Unmapped => self.create_l2table(addr.l1_index)?,
            _ => return Err(MapError::Remap),
        };

        if l2_table[addr.l2_index].get_type() != L2EntryType::Unmapped {
            return Err(MapError::Remap);
        }

        l2_table[addr.l2_index].set_phys(phys, L2EntryType::Small);
        l2_table[addr.l2_index].set_perm(perm);
        Ok(())
    }

    fn get_l1(&mut self, l1_index: usize) -> &mut Entry {
        &mut self.table[l1_index]
    }

    /// Makes sure that the second level table at `l1_index` is mapped and accessible.
    fn create_l2table(&mut self, l1_index: usize) -> Result<&mut SeconLevelTable> {
        let frame = kalloc::alloc_frame();
        // Since a frame returned by calloc is bigger than a single second layer table, we use the
        // new block to map the several tables surrounding the table needed.
        const L2_TABLES_PER_BLOCK: usize =
            size_of::<kalloc::Block>() / size_of::<SeconLevelTable>();
        let base_index = l1_index.align_down(L2_TABLES_PER_BLOCK);

        for (i, entry) in self.table[base_index..base_index + L2_TABLES_PER_BLOCK]
            .iter_mut()
            .enumerate()
        {
            match entry.get_type() {
                EntryKind::Unmapped => (),
                _ => return Err(MapError::Remap),
            };
            entry.set_l2_table(frame + (i * size_of::<SeconLevelTable>()), 0);
        }
        match self.table[l1_index].get_type() {
            EntryKind::SeconLevelTable(table) => Ok(TranslationTable::phys_to_virt(&table)),
            _ => panic!("Entry isn't second-level-table after creation"),
        }
    }

    pub fn apply(self) {
        crate::arch::set_ttbr0(self.table.as_ptr() as usize);
    }

    pub fn virt_to_phys(&self, virt: usize) -> Option<usize> {
        let parts = AddrParts::from(virt);
        let entry = &self.table[parts.l1_index];
        match entry.get_type() {
            EntryKind::Unmapped => None,
            EntryKind::Section(section_base) => Some(section_base.addr() + parts.section_offset()),
            EntryKind::SeconLevelTable(l2_table_phys) => {
                let l2_table = TranslationTable::phys_to_virt(&l2_table_phys);
                let l2_entry = &l2_table[parts.l2_index];
                l2_entry.get_phys().map(|addr| addr + parts.page_offset)
            }
            _ => panic!("Unsupported entry type"),
        }
    }

    pub fn unmap(&mut self, range: Range<usize>) {
        const SECTION_SIZE: usize = size_of::<Section>();
        let range = StepRange::align_from(range, SECTION_SIZE);
        for addr in range {
            let parts = AddrParts::from(addr);
            let entry = &mut self.table[parts.l1_index];
            match entry.get_type() {
                EntryKind::Unmapped => (),
                EntryKind::Section(_) => entry.unmap(),
                EntryKind::SeconLevelTable(_) => unimplemented!(),
                _ => panic!("Unsupported entry type"),
            }
        }
    }
}
