use core::mem::size_of;
use core::ops::Range;

use crate::console::println;
use crate::error::{Error, Result};
use crate::memory_model::phys_to_virt_mut;
use crate::mmu::addr_parts::{AddrParts, Offset};
use crate::mmu::entry::{Entry, EntryKind, SeconLevelTable, Section};
use crate::mmu::l2entry::L2EntryType;
use crate::mmu::PagePerm;
use crate::num::{AlignDown, AlignUp};
use crate::phys::Phys;
use crate::step_range::StepRange;
use crate::{heap, memory_model};

pub const SMALL_PAGE_SIZE: usize = 4096;
const L1_ENTRY_COUNT: usize = 2096;

type L1Table = [Entry; L1_ENTRY_COUNT];

enum AddressSpace {
    Kernel,
    User,
}

pub struct TranslationTable<'a> {
    table: &'a mut L1Table,
    address_space: AddressSpace,
}

impl<'a> TranslationTable<'a> {
    pub fn get_kernel() -> Self {
        let base = crate::arch::get_ttbr1();
        Self {
            table: unsafe { &mut *(base as *mut L1Table) },
            address_space: AddressSpace::Kernel,
        }
    }

    fn get_offset(&self, addr: usize) -> Result<Offset> {
        let range = match self.address_space {
            AddressSpace::Kernel => 0x8000_0000..0xffff_ffff,
            AddressSpace::User => 0x0000_0000..0x7fff_ffff,
        };
        if !range.contains(&addr) {
            Err(Error::OutOfRange)
        } else {
            Ok(Offset(addr - range.start))
        }
    }

    fn offset_to_virt(&self, offset: Offset) -> usize {
        let address_range_base = match self.address_space {
            AddressSpace::Kernel => 0x8000_0000,
            AddressSpace::User => 0x0000_0000,
        };
        address_range_base + offset.0
    }

    pub fn new(address_space: AddressSpace) -> Result<Self> {
        Ok(Self {
            table: crate::memory_model::phys_to_virt_mut(&heap::alloc::<L1Table>()?),
            address_space,
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
            let parts = AddrParts::from(self.get_offset(virt)?);
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
            let addr = AddrParts::from(self.get_offset(virt)?);
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

    pub fn apply_kernel(self) {
        crate::arch::set_ttbr1(self.table.as_ptr() as usize);
    }

    pub fn apply_user(&self) {
        crate::arch::set_ttbr0(self.table.as_ptr() as usize);
    }

    pub fn virt_to_phys(&self, virt: usize) -> Option<usize> {
        let parts = AddrParts::from(self.get_offset(virt).ok()?);
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

    pub fn seek_hole(&self, addr: usize) -> Result<Offset> {
        let mut addr = addr.align_down(SMALL_PAGE_SIZE);
        loop {
            let offset = self.get_offset(addr)?;
            let parts = AddrParts::from(offset);
            let entry = &self.table[parts.l1_index];
            match entry.get_type() {
                EntryKind::Unmapped => return Ok(offset),
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
                            return Ok(self.get_offset(addr)?);
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
            let Ok(offset) = self.get_offset(addr) else {
                return;
            };
            let parts = AddrParts::from(offset);
            let entry = &mut self.table[parts.l1_index];
            match entry.get_type() {
                EntryKind::Unmapped => (),
                EntryKind::Section(_) => entry.unmap(),
                EntryKind::SeconLevelTable(_) => unimplemented!(),
                _ => panic!("Unsupported entry type"),
            }
        }
    }

    pub fn map_device<T>(&mut self, device: Phys<T>) -> Result<*mut T> {
        let start = device.addr().align_down(SMALL_PAGE_SIZE);
        let end = (device.addr() + size_of::<T>()).align_up(SMALL_PAGE_SIZE);
        let size = end - start;
        let mut candidate = memory_model::DEVICE_VIRT;
        loop {
            let candidate = self.offset_to_virt(self.seek_hole(candidate)?);
            self.map(
                candidate,
                device.addr(),
                size,
                PagePerm::KernOnly,
                false,
                true,
            )?;
            break Ok(candidate as *mut T);
        }
    }

    pub fn seek_next_region(&self, _seek_after: usize) -> Option<usize> {
        todo!();
    }
}
