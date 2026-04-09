use core::mem::size_of;
use core::ops::Range;
use core::ptr::NonNull;

use crate::console::log;
use crate::error::{Error, Result};
use crate::heap;
use crate::mmu::addr_parts::{AddrParts, Offset};
use crate::mmu::entry::{Entry, EntryKind, SeconLevelTable, Section};
use crate::mmu::l2entry::L2EntryType;
use crate::mmu::PagePerm;
use crate::num::{AlignDown, AlignUp};
use crate::phys::{Phys, PhysMut};
use crate::step_range::StepRange;

use super::entry::EntryKindMut;
use super::PAGE_SIZE;

const L1_ENTRY_COUNT: usize = 2096;

#[repr(align(8192))]
pub(super) struct L1Table(pub(super) [Entry; L1_ENTRY_COUNT]);

#[derive(PartialEq, Clone, Copy)]
pub enum AddressSpace {
    Kernel,
    User,
}

pub struct TranslationTable<'a> {
    table: &'a mut L1Table,
    address_space: AddressSpace,
}

impl<'a> TranslationTable<'a> {
    pub(super) fn new(table: &'a mut L1Table, address_space: AddressSpace) -> TranslationTable<'a> {
        Self {
            table,
            address_space,
        }
    }
    pub fn get_kernel() -> Self {
        let base: PhysMut<L1Table> = (crate::arch::get_ttbr1() as *mut L1Table).into();
        Self {
            table: unsafe { &mut *(base.into_virt()) },
            address_space: AddressSpace::Kernel,
        }
    }

    fn get_range(&self) -> Range<usize> {
        match self.address_space {
            AddressSpace::Kernel => 0x8000_0000..0xffff_ffff,
            AddressSpace::User => 0x0000_0000..0x7fff_ffff,
        }
    }

    fn get_offset(&self, addr: usize) -> Result<Offset> {
        let range = self.get_range();
        if !range.contains(&addr) {
            Err(Error::OutOfRange)
        } else {
            Ok(Offset(addr - range.start))
        }
    }

    fn get_virt(&self, offset: Offset) -> usize {
        self.get_range().start + offset.0
    }

    fn offset_to_virt(&self, offset: Offset) -> usize {
        let address_range_base = match self.address_space {
            AddressSpace::Kernel => 0x8000_0000,
            AddressSpace::User => 0x0000_0000,
        };
        address_range_base + offset.0
    }

    #[allow(unused)]
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
            let entry = self.get_l1(parts.l1_index());

            match entry.get_type() {
                EntryKind::Unmapped => (),
                _ => return Err(Error::Remap),
            };

            entry.set_section(phys, perm, 0);
        }
        Ok(())
    }

    pub(super) fn map(
        &mut self,
        virt: usize,
        phys: usize,
        len: usize,
        perm: PagePerm,
        cachable: bool,
        bufferable: bool,
    ) -> Result<()> {
        let virt_range = StepRange::new(virt, virt + len, PAGE_SIZE);
        let phys_range = StepRange::new(phys, phys + len, PAGE_SIZE);

        log!(
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
        let entry = self.get_l1(addr.l1_index());

        let l2_table = match entry.get_type_mut() {
            EntryKindMut::SeconLevelTable(l2_table) => unsafe { &mut *l2_table.into_virt() },
            EntryKindMut::Unmapped => self.create_l2table(addr.l1_index())?,
            _ => return Err(Error::Remap),
        };

        if l2_table[addr.l2_index()].get_type() != L2EntryType::Unmapped {
            return Err(Error::Remap);
        }

        l2_table[addr.l2_index()].set_phys(phys, L2EntryType::Small, cachable, bufferable);
        l2_table[addr.l2_index()].set_perm(perm);
        Ok(())
    }

    fn get_l1(&mut self, l1_index: usize) -> &mut Entry {
        &mut self.table.0[l1_index]
    }

    /// Makes sure that the second level table at `l1_index` is mapped and accessible.
    fn create_l2table(&mut self, l1_index: usize) -> Result<&mut SeconLevelTable> {
        let new_l2_table = heap::alloc::<SeconLevelTable>()?;
        let entry = &mut self.table.0[l1_index];
        match entry.get_type() {
            EntryKind::Unmapped => (),
            _ => return Err(Error::Remap),
        };
        entry.set_l2_table(PhysMut::from_virt(new_l2_table), 0);
        // TODO Ok(phys_to_virt(frame))
        match self.table.0[l1_index].get_type_mut() {
            EntryKindMut::SeconLevelTable(table) => Ok(unsafe { &mut *table.into_virt() }),
            _ => panic!("Entry isn't second-level-table after creation"),
        }
    }

    #[allow(unused)]
    pub fn apply_kernel(self) {
        crate::arch::set_ttbr1(self.table.0.as_ptr() as usize);
    }

    fn seek_hole(&self, offset: Offset) -> Result<Offset> {
        let offset = Offset(offset.0.align_down(PAGE_SIZE));
        let mut parts = AddrParts::from(offset);
        loop {
            let entry = &self.table.0[parts.l1_index()];
            match entry.get_type() {
                EntryKind::Unmapped => return Ok(Offset(parts.addr())),
                EntryKind::Section(_) => {
                    parts.try_add(size_of::<Section>())?;
                }
                EntryKind::SeconLevelTable(l2_table) => {
                    let l2_table = unsafe { &*l2_table.into_virt() };
                    for entry in &l2_table[parts.l2_index()..] {
                        if entry.get_type() != L2EntryType::Unmapped {
                            parts.try_add(PAGE_SIZE)?;
                        } else {
                            return Ok(Offset(parts.addr()));
                        }
                    }
                }
                _ => panic!("Unsupported entry type"),
            };
        }
    }

    fn seek_mapped(&self, offset: Offset, limit: usize) -> Option<Offset> {
        let mut parts = AddrParts::from(offset);
        'outer: loop {
            let entry = &self.table.0[parts.l1_index()];
            match entry.get_type() {
                EntryKind::Unmapped => parts.try_add(PAGE_SIZE).ok()?,
                EntryKind::SuperSection | EntryKind::Section(_) => {
                    break;
                }
                EntryKind::SeconLevelTable(l2_table) => {
                    let l2_table = unsafe { &*l2_table.into_virt() };
                    for entry in &l2_table[parts.l2_index()..] {
                        if entry.get_type() == L2EntryType::Unmapped {
                            parts.try_add(PAGE_SIZE).ok()?;
                        } else {
                            break 'outer;
                        }
                    }
                }
            };
            if parts.addr() - offset.0 > limit {
                break;
            }
        }
        Some(Offset(parts.addr()))
    }

    pub(super) fn unmap_all(&mut self) {
        if self.address_space == AddressSpace::Kernel {
            panic!("Refusing to unmap the kernel");
        }
        for entry in &mut self.table.0 {
            match entry.get_type_mut() {
                EntryKindMut::Unmapped => (),
                EntryKindMut::Section(_) => todo!(),
                EntryKindMut::SeconLevelTable(l2_table) => {
                    Self::unmap_second_level(l2_table);
                    entry.unmap();
                }
                _ => panic!("Unsupported entry type"),
            }
        }
    }

    fn unmap_second_level(l2_table: PhysMut<SeconLevelTable>) {
        let table_ptr = l2_table.into_virt();
        let l2_table = unsafe { &mut *table_ptr };
        for entry in &mut l2_table.0 {
            let phys = match entry.get_type() {
                L2EntryType::Unmapped => None,
                L2EntryType::Small => entry.get_phys(),
                L2EntryType::Large => todo!(),
            };
            let Some(phys) = phys else {
                continue;
            };
            heap::dealloc(PhysMut::<crate::mmu::Page>::from(phys).into_virt());
            entry.unmap()
        }
        heap::dealloc(table_ptr);
    }

    pub fn map_device<T>(&mut self, device: Phys<T>) -> Result<NonNull<T>> {
        let start = device.addr().align_down(PAGE_SIZE);
        let offset = device.addr() - start;
        let end = (device.addr() + size_of::<T>()).align_up(PAGE_SIZE);
        let size = end - start;
        let candidate = self.offset_to_virt(self.seek_hole(Offset(0))?);
        self.map(
            candidate,
            device.addr(),
            size,
            PagePerm::KernOnly,
            false,
            true,
        )?;
        Ok(NonNull::<T>::new((candidate + offset) as *mut T).unwrap())
    }

    pub fn map_stack(&mut self, phys: Phys<[u8]>, perm: PagePerm) -> Result<NonNull<()>> {
        let start = self.find_hole(phys.len())?;
        let stack_bottom = self.get_virt(start);
        self.map(stack_bottom, 0, PAGE_SIZE, PagePerm::NoOne, false, false)?;
        self.map(
            stack_bottom + PAGE_SIZE,
            phys.addr(),
            phys.len(),
            perm,
            true,
            false,
        )?;
        Ok(NonNull::new((stack_bottom + PAGE_SIZE) as *mut ()).unwrap())
    }

    pub fn map_memory(&mut self, phys: Phys<[u8]>, perm: PagePerm) -> Result<&'static [u8]> {
        let page_offset = Self::page_offset(&phys);
        let size = phys.len() + page_offset;
        let table_offset = self.find_hole(size)?;
        let virt = self.get_virt(table_offset);
        self.map(virt, phys.addr(), size, perm, true, true)?;
        let virt = phys.with_addr(virt + page_offset);
        Ok(unsafe { &*virt })
    }

    fn page_offset<T>(phys: &Phys<[T]>) -> usize {
        phys.addr() & 0xfff
    }

    fn find_hole(&mut self, size: usize) -> Result<Offset> {
        let mut start = Offset(0);
        loop {
            start = self.seek_hole(start)?;
            let end = self.seek_mapped(start, size + PAGE_SIZE);
            let hole_size = match end {
                Some(end) => end - start,
                None => 0x8000_0000 - start.0,
            };
            if hole_size > size + PAGE_SIZE && start.0 != 0 {
                break;
            } else {
                match end {
                    Some(end) => start = end,
                    None => return Err(Error::OutOfMem),
                }
            }
        }
        Ok(start)
    }
}
