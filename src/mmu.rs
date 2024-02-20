use crate::kalloc;
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

enum MapError {
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

impl AddrParts {
    fn index_in_slave(&self) -> usize {
        (self.l1_index << 8 | self.l2_index) / SMALL_PAGE_SIZE
    }
}

#[repr(C)]
struct L2Entry {
    value: usize,
}

pub enum PagePerm {
    KernNone,
    UserNone,
    UserRo,
    UserRw,
}

enum L2EntryType {
    SMALL,
    LARGE,
}

#[inline]
fn translate_perm(perm: PagePerm) -> usize {
    match perm {
        PagePerm::KernNone => 0,
        PagePerm::UserNone => 1,
        PagePerm::UserRo => 2,
        PagePerm::UserRw => 3,
    }
}

impl L2Entry {
    #[inline]
    pub fn set_phys(&mut self, phys: usize, entry_type: L2EntryType) {
        let mask = match entry_type {
            L2EntryType::SMALL => 0xfffff000,
            L2EntryType::LARGE => 0xffff0000,
        };
        self.value = phys & mask;
    }

    #[inline]
    pub fn get_phys(&self) -> *mut u8 {
        let mask = match self.get_type() {
            L2EntryType::SMALL => 0xfffff000,
            L2EntryType::LARGE => 0xffff0000,
        };
        (self.value & mask) as *mut u8
    }

    #[inline]
    fn get_type(&self) -> L2EntryType {
        if self.value & 2 == 0 {
            L2EntryType::SMALL
        } else {
            L2EntryType::LARGE
        }
    }

    #[inline]
    fn set_perm(&mut self, perm: PagePerm) {
        self.value |= translate_perm(perm);
    }
}
const_assert!(size_of::<L2Entry>() == 4);

struct Entry {
    value: usize,
}

impl Entry {
    fn as_table(&self) -> &L2Table {
        assert!(self.value & 0b11 == 0b01);
        unsafe { &*(self.value as *const L2Table) }
    }

    fn as_table_mut(&self) -> &mut L2Table {
        assert!(self.value & 0b11 == 0b01);
        unsafe { &mut *(self.value as *mut L2Table) }
    }

    fn is_mapped(&self) -> bool {
        self.value & 0x3 != 0
    }

    fn set_section(&mut self, phys: usize, perm: PagePerm) {
        self.value = (phys & 0xfff00000) | translate_perm(perm);
    }

    fn unmap(&mut self) {
        self.value = 0;
    }
}

#[repr(C)]
struct L2Table {
    entries: [L2Entry; 256],
}
const_assert!(size_of::<L2Table>() == 1024);

impl L2Table {
    pub fn find_unmapped(&self, len: usize) -> Option<usize> {
        let mut total = 0usize;
        let mut offset = Some(0usize);
        for (i, entry) in self.entries.iter().enumerate() {
            if entry.value != 0 {
                total = 0;
                offset = None;
                continue;
            }

            if offset.is_none() {
                offset = Some(i);
            }
            total += SMALL_PAGE_SIZE;

            if total > len {
                return offset;
            }
        }
        None
    }
}

#[repr(C)]
pub struct BasicTranslationTable {
    entries: [Entry; 4096],
}

const L2_TABLES_PER_BLOCK: usize = kalloc::BLOCK_SIZE / size_of::<L2Table>();

impl BasicTranslationTable {
    // pub fn new(current_table: &mut Self) -> Option<*mut Self> {
    //     let virt = current_table.find_unmapped(kalloc::BLOCK_SIZE)?;
    //     let ptr = kalloc::alloc_frame();
    //     current_table.map(ptr, virt);
    //     let page = virt as *mut u8;
    //     unsafe {
    //         page.write_bytes(0, kalloc::BLOCK_SIZE);
    //     }
    //     let new_table = page as *mut Self;
    //     // new_table.map(page,
    // }
    pub fn virt_to_phys(&self, virt: usize) -> Option<usize> {
        None
    }

    pub fn replace_section(&mut self, virt: usize, phys: usize) {
        let entry = self.get_entry(virt);
        // TODO free_frame if this is a second level table
        entry.unmap();

        entry.set_section(phys, PagePerm::UserNone);
    }

    fn get_entry(&mut self, virt: usize) -> &mut Entry {
        let parts = AddrParts::from(virt);
        &mut self.entries[parts.l1_index]
    }

    // pub fn map(&mut self, phys: usize, virt: usize) {
    //     let entry_index = virt >> 20;
    //     let l2_entry_index = (virt >> 12) & 0xff;
    //     let l2_table = self.get_entry(entry_index).as_table_mut();
    //     let l2_entry = l2_table.get_entry(l2_entry_index);
    // }

    // pub fn get_entry(&mut self, i: usize) -> &mut Entry {
    //     self.map_entry(i);
    //     self.get_entry_unchecked(i)
    // }

    // pub fn map_entry(&mut self, i: usize) {
    //     if self.get_entry_unchecked(i).value == 0 {
    //         return;
    //     }
    //     let ptr = kalloc::alloc_frame();
    //     unsafe {
    //         ptr.write_bytes(0, kalloc::BLOCK_SIZE);
    //     }
    //     let addr = ptr as usize;
    //     let base_index = i % L2_TABLES_PER_BLOCK;
    //     for j in 0..L2_TABLES_PER_BLOCK {
    //         self.entries[base_index + j].value = (addr + (j * size_of::<L2Table>())) & 0xfffffc00;
    //     }
    // }

    // pub fn get_entry_unchecked(&mut self, i: usize) -> &mut Entry {
    //     let entry = &mut self.entries[i];
    //     entry
    // }

    // pub fn find_unmapped(&self, len: usize) -> Option<usize> {
    //     // TODO improve to look and end and start of adjacent l2 entries
    //     for (i, entry) in self.entries.iter().enumerate() {
    //         if let Some(entry_index) = entry.as_table().find_unmapped(len) {
    //             return Some((i << 20) | (entry_index << 12));
    //         }
    //     }
    //     None
    // }
}

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

impl TranslationTable {
    /// The translateion table must be in use
    pub fn map_page(&mut self, phys: usize, virt: usize) {
        let parts = AddrParts::from(virt);
        let l2 = self.get_l2_table(&parts); // address accessible by us to edit the table
        let l2_entry = &mut l2.entries[parts.l2_index];
        l2_entry.set_phys(phys, L2EntryType::SMALL);
    }

    /// Get the Second level table associated with a virtual address.
    ///
    /// If the table is not mapped, a new table allocated.
    /// It's physicall address is and asssociates in the first level table.
    /// It's physicall address is also mapped in the slave table, it's content
    /// can we written and read.
    fn get_l2_table(&mut self, addr_parts: &AddrParts) -> &mut L2Table {
        let index_in_slave = addr_parts.index_in_slave();

        if !self.does_l2_exist(addr_parts) {
            let frame_phys_addr = kalloc::alloc_frame();
            self.map_frame_to_l1_table(frame_phys_addr, addr_parts.l1_index);

            self.map_frame_to_slave_table(frame_phys_addr, index_in_slave);
        }

        self.get_l2_table_unchecked(index_in_slave)
    }

    fn does_l2_exist(&mut self, addr_parts: &AddrParts) -> bool {
        self.get_l1_table().entries[addr_parts.l1_index].value != 0
    }

    /// Associate the physicall address of the table to the first level table.
    fn map_frame_to_l1_table(&self, frame: usize, l1_index: usize) {
        let base_index = l1_index % L2_TABLES_PER_BLOCK;
        let l1_table = self.get_l1_table();
        for j in 0..L2_TABLES_PER_BLOCK {
            l1_table.entries[base_index + j].value =
                (frame + (j * size_of::<L2Table>())) & 0xfffffc00;
        }
    }

    /// Associate the block with some virtual address in the area controlled by the slave table
    fn map_frame_to_slave_table(&self, frame: usize, index_in_slave: usize) {
        let slave_table = self.get_slave_table();
        for j in 0..L2_TABLES_PER_BLOCK {
            slave_table.entries[index_in_slave + j]
                .set_phys(frame + (j * size_of::<L2Table>()), L2EntryType::SMALL);
        }
    }

    fn get_l2_table_unchecked(&self, table_index: usize) -> &mut L2Table {
        let table_base = self.range.end - (SECTION_SIZE * 2) - MAPPABLE_L2_TABLES_SIZE;
        let base_ptr = table_base as *mut L2Table;
        unsafe { &mut *(base_ptr.add(table_index)) }
    }

    fn get_slave_table(&self) -> &mut L2Table {
        let table_addr = self.range.end - (SECTION_SIZE * 2);
        let table_ptr = table_addr as *mut L2Table;
        unsafe { table_ptr.as_mut().unwrap() }
    }

    pub fn get_l1_table(&self) -> &mut BasicTranslationTable {
        let l1_addr = self.range.end - SECTION_SIZE;
        let l1_ptr = l1_addr as *mut BasicTranslationTable;
        unsafe { l1_ptr.as_mut().unwrap() }
    }
}

pub struct TranslationTableBuilder<'a> {
    current_table: &'a mut BasicTranslationTable,
    new_table: &'a mut BasicTranslationTable,
    table_phys: usize,
    slave_table: &'a mut L2Table,
    slave_phys: usize,
}

impl<'a> TranslationTableBuilder<'a> {
    const KERN_L1_TABLE_ADDR: usize = 0xfff00000;
    const KERN_SLAVE_TABLE_ADDR: usize = Self::KERN_L1_TABLE_ADDR - SECTION_SIZE;

    pub fn new(current_table: &'a mut BasicTranslationTable) -> Option<Self> {
        let table_phys = kalloc::alloc_frame();
        let slave_phys = kalloc::alloc_frame();
        current_table.replace_section(Self::KERN_L1_TABLE_ADDR, table_phys);
        current_table.replace_section(Self::KERN_SLAVE_TABLE_ADDR, slave_phys);

        let new_table = unsafe { &mut *(Self::KERN_L1_TABLE_ADDR as *mut BasicTranslationTable) };
        let slave_table = unsafe { &mut *(Self::KERN_SLAVE_TABLE_ADDR as *mut L2Table) };
        Some(Self {
            current_table,
            new_table,
            table_phys,
            slave_table,
            slave_phys,
        })
    }

    pub fn prepare_map(&mut self, virt: usize, phys: usize, len: usize) {
        let parts = AddrParts::from(virt);
        todo!();
    }

    pub fn apply(self) -> TranslationTable {
        todo!();
    }
}
