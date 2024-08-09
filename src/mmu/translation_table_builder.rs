use crate::console::println;
use crate::kalloc;
use crate::step_range::StepRange;
use core::arch::asm;
use core::mem::size_of;
use static_assertions::const_assert;

use crate::mmu::addr_parts::AddrParts;
use crate::mmu::entry::{Entry, EntryKind};
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

// TODO delete
// pub fn replace_section(table: EntryPtr, virt: usize, phys: usize) {
//     let parts = AddrParts::from(virt);
//     let entry = unsafe { table.add(parts.l1_index).as_mut().unwrap() };
//     // TODO free_frame if this is a second level table
//     entry.unmap();

//     entry.set_section(phys, PagePerm::KernOnly, 0);
// }

// pub fn find_unmapped(self: &L2Table, len: usize) -> Option<usize> {
//     let mut total = 0usize;
//     let mut offset = Some(0usize);
//     for (i, entry) in self.entries.iter().enumerate() {
//         if entry.value != 0 {
//             total = 0;
//             offset = None;
//             continue;
//         }

//         if offset.is_none() {
//             offset = Some(i);
//         }
//         total += SMALL_PAGE_SIZE;

//         if total > len {
//             return offset;
//         }
//     }
//     None
// }

const L1_ENTRY_COUNT: usize = 4096;
const L1_TABLE_SIZE: usize = L1_ENTRY_COUNT * size_of::<Entry>();
const L1_TABLES_PER_BLOCK: usize = kalloc::BLOCK_SIZE / L1_TABLE_SIZE;

const L2_TABLE_SIZE: usize = Entry::L2_ENTRY_COUNT * size_of::<L2Entry>();
const L2_TABLES_PER_BLOCK: usize = kalloc::BLOCK_SIZE / L2_TABLE_SIZE;

// The slave second level table maps is filled with L2 entries, each cappable of mapping a
// small-page.
const MAPPABLE_L2_TABLES_SIZE: usize =
    (kalloc::BLOCK_SIZE / size_of::<L2Entry>()) * SMALL_PAGE_SIZE;
const_assert!(MAPPABLE_L2_TABLES_SIZE == 16 * 1024 * 1024);

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
    pub fn new() -> Option<Self> {
        let table_phys = kalloc::alloc_frame();

        let table =
            unsafe { core::slice::from_raw_parts_mut(table_phys as *mut Entry, L1_ENTRY_COUNT) };
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
        let base_index = align_down(l1_index, L2_TABLES_PER_BLOCK);

        for (i, entry) in self.table[base_index..base_index + L2_TABLES_PER_BLOCK]
            .iter_mut()
            .enumerate()
        {
            match entry.get_type() {
                EntryKind::Unmapped => (),
                _ => return Err(MapError::Remap),
            };
            entry.set_l2_table(frame + (i * L2_TABLE_SIZE), 0);
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
