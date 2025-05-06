use core::mem::size_of;
use core::ops::Range;

use crate::console::println;
use crate::error::{Error, Result};
use crate::memory_model::phys_to_virt_mut;
use crate::mmu::addr_parts::AddrParts;
use crate::mmu::entry::{Entry, EntryKind, SeconLevelTable, Section};
use crate::mmu::l2entry::L2EntryType;
use crate::mmu::PagePerm;
use crate::num::{AlignDown, AlignUp};
use crate::phys::Phys;
use crate::step_range::StepRange;
use crate::{heap, memory_model};

pub const SMALL_PAGE_SIZE: usize = 4096;
const L1_ENTRY_COUNT: usize = 4096;

type L1Table = [Entry; L1_ENTRY_COUNT];

pub struct TranslationTable<'a> {
    table: &'a mut L1Table,
}

impl<'a> TranslationTable<'a> {
    pub fn from_base(base: usize) -> Self {
        Self {
            table: unsafe { &mut *(base as *mut L1Table) },
        }
    }

    pub fn new() -> Result<Self> {
        Ok(Self {
            table: crate::memory_model::phys_to_virt_mut(&heap::alloc::<L1Table>()?),
        })
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
                _ => return Err(Error::Remap),
            };

            entry.set_section(phys, perm, 0);
        }
        Ok(())
    }

    pub fn map(
        &mut self,
        virt: usize,
        phys: usize,
        len: usize,
        perm: PagePerm,
        cachable: bool,
        bufferable: bool,
    ) -> Result<()> {
        let virt_range = StepRange::new(virt, virt + len, SMALL_PAGE_SIZE);
        let phys_range = StepRange::new(phys, phys + len, SMALL_PAGE_SIZE);

        println!(
            "mapping virt 0x{:x}[0..0x{:x}] to phys 0x{:x}",
            virt, len, phys
        );

        for (virt, phys) in virt_range.zip(phys_range) {
            let addr = AddrParts::from(virt);
            self.map_once(&addr, phys, perm, cachable, bufferable)?;
        }
        Ok(())
    }

    fn map_once(
        &mut self,
        addr: &AddrParts,
        phys: usize,
        perm: PagePerm,
        cachable: bool,
        bufferable: bool,
    ) -> Result<()> {
        let entry = self.get_l1(addr.l1_index);

        let l2_table = match entry.get_type() {
            EntryKind::SeconLevelTable(l2_table) => phys_to_virt_mut(&l2_table),
            EntryKind::Unmapped => self.create_l2table(addr.l1_index)?,
            _ => return Err(Error::Remap),
        };

        if l2_table[addr.l2_index].get_type() != L2EntryType::Unmapped {
            return Err(Error::Remap);
        }

        l2_table[addr.l2_index].set_phys(phys, L2EntryType::Small, cachable, bufferable);
        l2_table[addr.l2_index].set_perm(perm);
        Ok(())
    }

    fn get_l1(&mut self, l1_index: usize) -> &mut Entry {
        &mut self.table[l1_index]
    }

    /// Makes sure that the second level table at `l1_index` is mapped and accessible.
    fn create_l2table(&mut self, l1_index: usize) -> Result<&mut SeconLevelTable> {
        let new_l2_table = heap::alloc::<SeconLevelTable>()?;
        let entry = &mut self.table[l1_index];
        match entry.get_type() {
            EntryKind::Unmapped => (),
            _ => return Err(Error::Remap),
        };
        entry.set_l2_table(new_l2_table, 0);
        // TODO Ok(phys_to_virt(frame))
        match self.table[l1_index].get_type() {
            EntryKind::SeconLevelTable(table) => Ok(phys_to_virt_mut(&table)),
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
                let l2_table = phys_to_virt_mut(&l2_table_phys);
                let l2_entry = &l2_table[parts.l2_index];
                l2_entry.get_phys().map(|addr| addr + parts.page_offset)
            }
            _ => panic!("Unsupported entry type"),
        }
    }

    pub fn seek_hole(&self, addr: usize) -> Option<usize> {
        let mut addr = addr.align_down(SMALL_PAGE_SIZE);
        loop {
            let parts = AddrParts::from(addr);
            let entry = &self.table[parts.l1_index];
            match entry.get_type() {
                EntryKind::Unmapped => return Some(addr),
                EntryKind::Section(_) => {
                    if parts.l2_index == 0 {
                        addr += size_of::<Section>();
                    } else {
                        addr = addr.align_up(size_of::<Section>());
                    }
                }
                EntryKind::SeconLevelTable(l2_table) => {
                    let l2_table = unsafe { &*memory_model::phys_to_virt(&l2_table) };
                    for entry in &l2_table[parts.l2_index..] {
                        if entry.get_type() != L2EntryType::Unmapped {
                            addr += SMALL_PAGE_SIZE;
                        } else {
                            return Some(addr);
                        }
                    }
                }
                _ => panic!("Unsupported entry type"),
            };
        }
    }

    pub fn seek_mapped(&self, _addr: usize) -> Option<usize> {
        todo!();
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

    pub fn map_device<T>(&mut self, device: Phys<T>) -> Option<*mut T> {
        let start = device.addr().align_down(SMALL_PAGE_SIZE);
        let end = (device.addr() + size_of::<T>()).align_up(SMALL_PAGE_SIZE);
        let size = end - start;
        let mut candidate = memory_model::DEVICE_VIRT;
        loop {
            let candidate_end = self.seek_hole(candidate)?;
            if (candidate_end - candidate) >= size {
                self.map(
                    candidate,
                    device.addr(),
                    size,
                    PagePerm::KernOnly,
                    false,
                    true,
                );
                break Some(start as *mut T);
            } else {
                candidate = self.seek_next_region(candidate_end)?;
            }
        }
    }

    pub fn seek_next_region(&self, _seek_after: usize) -> Option<usize> {
        todo!();
    }
}
