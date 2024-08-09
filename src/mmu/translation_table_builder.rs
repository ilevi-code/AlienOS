use crate::console::println;
use crate::kalloc;
use crate::step_range::StepRange;
use core::arch::asm;
use core::mem::size_of;

use crate::mmu::addr_parts::AddrParts;
use crate::mmu::entry::{Entry, EntryKind, SeconLevelTable};
use crate::mmu::error::{MapError, Result};
use crate::mmu::l2entry::{L2Entry, L2EntryType};
use crate::mmu::PagePerm;

fn div_ceil(left: usize, other: usize) -> usize {
    (left + other - 1) / other
}

fn align_down(val: usize, align: usize) -> usize {
    let mask = align - 1;
    val & (!mask)
}

pub const SMALL_PAGE_SIZE: usize = 4096;
const SECTION_SIZE: usize = 1024 * 1024;
const USER_END: usize = 0x80000000;

pub struct TranslationTableBuilder<'a> {
    table: &'a mut [Entry],
}

pub fn get_ttbr0() -> usize {
    let table;
    unsafe {
        asm!("MRC p15, 0, {table}, c2, c0, 0", table = out(reg) table);
    }
    table
}

impl<'a> TranslationTableBuilder<'a> {
    const L1_ENTRY_COUNT: usize = 4096;

    pub fn new() -> Option<Self> {
        let table_phys = kalloc::alloc_frame();

        let table = unsafe {
            core::slice::from_raw_parts_mut(
                table_phys as *mut Entry,
                TranslationTableBuilder::L1_ENTRY_COUNT,
            )
        };
        Some(Self { table })
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
            EntryKind::SeconLevelTable(l2_table) => l2_table,
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
    fn create_l2table(&mut self, l1_index: usize) -> Result<&mut [L2Entry]> {
        let frame = kalloc::alloc_frame();
        // Since a frame returned by calloc is bigger than a single second layer table, we use the
        // new block to map the several tables surrounding the table needed.
        const L2_TABLES_PER_BLOCK: usize = kalloc::BLOCK_SIZE / size_of::<SeconLevelTable>();
        let base_index = align_down(l1_index, L2_TABLES_PER_BLOCK);

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
            EntryKind::SeconLevelTable(table) => Ok(table),
            _ => panic!("Entry isn't second-level-table after creation"),
        }
    }

    pub fn apply(self) {
        unsafe {
            let table = self.table.as_ptr();
            asm!("MCR p15, 0, {table}, c2, c0, 0", table = in(reg) table);
        }
    }
}
