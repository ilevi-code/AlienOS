use crate::console::println;
use crate::kalloc;
use crate::step_range::StepRange;
use core::mem::size_of;
use static_assertions::const_assert;

// defined by linker script
extern "C" {
    static kernel_start: u8;
    static kernel_end: u8;
}

pub fn get_kernel_location() -> core::ops::Range<usize> {
    unsafe {
        let kernel_end_addr_virt = (&kernel_end as *const u8) as usize;
        let kernel_start_addr_virt = (&kernel_start as *const u8) as usize;
        kernel_start_addr_virt..kernel_end_addr_virt
    }
}

fn div_ceil(left: usize, other: usize) -> usize {
    (left + other - 1) / other
}

fn align_down(val: usize, align: usize) -> usize {
    let mask = align - 1;
    val & (!mask)
}

#[derive(Debug)]
pub enum MapError {
    Remap,
}

type Result<T> = core::result::Result<T, MapError>;

/// Upon boot the end of the kernel should be mapped in 1:1 mode, to allow creation of new
/// translation tables.
///
/// When creating new translation tables, calls to kalloc may (and probably will) return a non-mapped 16KB region. To
/// So we must assure the pages are indeed writable.
/// To this we do the following:
///
/// 1. A section entry that maps access to the First layer translation table.
///    We do not use the whole 1MB of the section, it just convienient.
///    Section do not require an accessible page
///    Base address: 0x7ff00000 (last 1MB of the table)
/// 2. Another section that is used for a Second layer table.
///    This table isn't used for only for mapping othe second layer table's frame, so can be
///    written by us.
///    Base address: 0x7fe00000 (second to last 1MB of the table)
///    The mapped block is 16KB of memory, allowing mapping 4096 entries of 4K each.
///    This is 16MB of memory acessible by us.
/// 3. Assuming the whole 16MB are also not used by the user - but by us for acessing even more
///    second layer tables. That's 4M second level entries, each one capable of mapping 4k.
///    That's 16GB of memory. We'll be fine from here on.
///    Size actually needed for memory of user-land:
///    (2Gb of memory) / (1M mappable size entry) = 2M
///
/// Behold:
//               +--> +-----------------------+
//               |    |      First table      |              0x80000000 - 0x7ff00000
//               |    |    virtual adddres    |         (1MB sectioon, first 16K are valid)
//               +--> +-----------------------+
// +---------+   |    |     Slave tables      |              0x7ff00000 - 0x7fe00000
// |         |   |    |    Maps next 16MB     |        (1MB sectioon, first 16K are valid)
// |  First  +---+    +-----------------------+
// |  Level  |   |    | Second level table's  |              0x7fe00000 - 0x7ee00000
// |  Table  |   +--> |  virtual addresses    |  (16MB, 4K granular, Controlled by the slave tables)
// |         |        +-----------------------+
// +---------+---+
//               +--> +-----------------------+
//                    |  User land mappings   |              0x7ee00000 - 0x0
//                    |         ...           |
//                    +-----------------------+

pub const SMALL_PAGE_SIZE: usize = 4096;
const SECTION_SIZE: usize = 1024 * 1024;
const USER_END: usize = 0x80000000;

struct AddrParts {
    l1_index: usize,
    l2_index: usize,
    page_offset: usize,
}

impl From<usize> for AddrParts {
    fn from(virt: usize) -> Self {
        Self {
            l1_index: virt >> 20,
            l2_index: (virt >> 12) & 0xff,
            page_offset: virt & 0xfff,
        }
    }
}

#[repr(C)]
struct L2Entry {
    value: usize,
}

#[derive(Clone, Copy)]
pub enum PagePerm {
    NoOne,
    KernOnly,
    UserRo,
    UserRw,
}

#[derive(PartialEq, Eq)]
enum L2EntryType {
    Unmapped,
    Small,
    Large,
}

#[inline]
fn translate_perm(perm: PagePerm) -> usize {
    match perm {
        PagePerm::NoOne => 0,
        PagePerm::KernOnly => 1,
        PagePerm::UserRo => 2,
        PagePerm::UserRw => 3,
    }
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
    fn get_type(&self) -> L2EntryType {
        match self.value & 0b11 {
            0 => L2EntryType::Unmapped,
            1 => L2EntryType::Small,
            _ => L2EntryType::Large,
        }
    }

    #[inline]
    fn set_perm(&mut self, perm: PagePerm) {
        self.value |= translate_perm(perm);
    }
}
const_assert!(size_of::<L2Entry>() == 4);

pub struct Entry {
    value: usize,
}

pub type EntryPtr = *mut Entry;

pub enum EntryType {
    Unmapped,
    SeconLevelTable,
    Section,
    SuperSection,
}

fn phys_to_virt<T>(addr: usize) -> *const T {
    const PHYS_START_IN_KERN_VIRT: usize = 0x80000000;
    const PHYS_START: usize = 0x40000000;
    (addr - PHYS_START + PHYS_START_IN_KERN_VIRT) as *const T
}

pub fn virt_to_phys(table: EntryPtr, virt: usize) -> Option<usize> {
    let parts = AddrParts::from(virt);
    let entry = unsafe { table.add(parts.l1_index).as_ref() }?;
    let l2_table = match entry.get_type() {
        EntryType::Unmapped => return None,
        EntryType::Section => return Some(entry.as_section()),
        EntryType::SeconLevelTable => entry.as_l2_table(),
        _ => panic!("Unsupported entry type"),
    };
    let l2_entry = &l2_table[parts.l2_index];
    l2_entry.get_phys().map(|addr| addr + parts.page_offset)
}

pub fn replace_section(table: EntryPtr, virt: usize, phys: usize) {
    let parts = AddrParts::from(virt);
    let entry = unsafe { table.add(parts.l1_index).as_mut().unwrap() };
    // TODO free_frame if this is a second level table
    entry.unmap();

    entry.set_section(phys, PagePerm::KernOnly, 0);
}

impl Entry {
    const SUPERSECTION_BIT: usize = 1 << 18;
    const SECTION_MASK: usize = 0xfff00000;
    const L2_TABLE_MASK: usize = 0xfffffc00;
    const L2_ENTRY_COUNT: usize = 256;

    const IGNORED_ENTRY_MAGIC: usize = 0b00;
    const SECOND_LEVEL_TABLE_MAGIC: usize = 0b01;
    const SECTION_MAGIC: usize = 0b10;

    pub fn get_type(&self) -> EntryType {
        match self.value & 0b11 {
            Self::IGNORED_ENTRY_MAGIC => EntryType::Unmapped,
            Self::SECOND_LEVEL_TABLE_MAGIC => EntryType::SeconLevelTable,
            Self::SECTION_MAGIC => {
                if self.is_supersection() {
                    EntryType::SuperSection
                } else {
                    EntryType::Section
                }
            }
            _ => panic!("Unsupported entry type"),
        }
    }

    fn as_l2_table(&self) -> &[L2Entry] {
        assert!(self.value & 0b11 == Self::SECOND_LEVEL_TABLE_MAGIC);
        unsafe {
            core::slice::from_raw_parts(phys_to_virt::<L2Entry>(self.value), Self::L2_ENTRY_COUNT)
        }
    }

    fn as_l2_table_mut(&self) -> Option<&mut [L2Entry]> {
        if self.value & 0b11 != Self::SECOND_LEVEL_TABLE_MAGIC {
            None
        } else {
            Some(unsafe {
                core::slice::from_raw_parts_mut(
                    phys_to_virt::<L2Entry>(self.value).cast_mut(),
                    Self::L2_ENTRY_COUNT,
                )
            })
        }
    }

    fn as_section(&self) -> usize {
        self.value & Self::SECTION_MASK
    }

    fn is_mapped(&self) -> bool {
        self.value & 0x3 != 0
    }

    fn set_section(&mut self, phys: usize, perm: PagePerm, domain: u8) {
        self.value = (phys & Self::SECTION_MASK)
            | (translate_perm(perm) << 10)
            | ((domain as usize) << 5)
            | (Self::SECTION_MAGIC);
    }

    fn set_l2_table(&mut self, phys: usize, domain: u8) {
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
const_assert!(size_of::<L2Entry>() == 4);
const_assert!(MAPPABLE_L2_TABLES_SIZE == 16 * 1024 * 1024);

pub struct TranslationTable {
    table_phys: usize,
    range: core::ops::Range<usize>,
}

struct TranslateTableIterator<'a> {
    addr: AddrParts,
    entreis: &'a mut [Entry],
}

impl<'a> Iterator for TranslateTableIterator<'a> {
    type Item = &'a mut L2Entry;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

pub struct TranslationTableBuilder<'a> {
    table: &'a mut [Entry],
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

        for (virt, phys) in virt_range.zip(phys_range) {
            let addr = AddrParts::from(virt);
            self.map_once(&addr, phys, perm)?;
            // use crate::console;
            // console::write("done");
        }
        Ok(())
    }

    fn map_once(&mut self, addr: &AddrParts, phys: usize, perm: PagePerm) -> Result<()> {
        let entry = self.get_l1(addr.l1_index);

        let l2_table = if let Some(l2_table) = entry.as_l2_table_mut() {
            l2_table
        } else {
            self.map_l2_at(addr.l1_index)?
                .as_l2_table_mut()
                .expect("Entry should contains L2 table after mapping")
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
    fn map_l2_at(&mut self, l1_index: usize) -> Result<&mut Entry> {
        let frame = kalloc::alloc_frame();
        let base_index = align_down(l1_index, L2_TABLES_PER_BLOCK);

        for (i, entry) in self.table[base_index..base_index + L2_TABLES_PER_BLOCK]
            .iter_mut()
            .enumerate()
        {
            if entry.is_mapped() {
                return Err(MapError::Remap);
            }
            entry.set_l2_table(frame + (i * L2_TABLE_SIZE), 0);
        }
        Ok(&mut self.table[l1_index])
    }

    fn old_map_l2(l2_table: &mut [L2Entry], l2_index: usize, perm: PagePerm) {
        let frame = kalloc::alloc_frame();
        let base_index = l2_index % L2_TABLES_PER_BLOCK;
        for j in 0..L2_TABLES_PER_BLOCK {
            l2_table[base_index + j].set_phys(frame + (j * L2_TABLE_SIZE), L2EntryType::Small);
            l2_table[base_index + j].set_perm(perm);
        }
    }

    pub fn apply(self) -> TranslationTable {
        todo!();
    }
}
