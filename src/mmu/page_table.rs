use crate::alloc::Box;
use crate::error::Result;
use crate::heap;
use crate::mmu::translation_table::{AddressSpace, L1Table};
use crate::mmu::{Page, PagePerm, TranslationTable};
use crate::phys::Phys;

pub struct PageTable {
    table: Box<L1Table>,
}

impl PageTable {
    pub fn new() -> Result<Self> {
        Ok(Self {
            table: Box::<L1Table>::zeroed()?,
        })
    }

    pub fn apply_user(&self) {
        crate::arch::set_ttbr0(Phys::from_virt(self.table.0.as_ptr()).addr());
    }

    pub fn map_memory(&mut self, phys: Phys<[u8]>, perm: PagePerm) -> Result<&'static [u8]> {
        self.as_translation_table().map_memory(phys, perm)
    }

    pub fn alloc_page(&mut self, virt: usize, perm: PagePerm) -> Result<&'static mut [u8]> {
        let page = heap::alloc::<Page>()?;
        self.as_translation_table().map(
            virt,
            Phys::from_virt(page).addr(),
            size_of::<Page>(),
            perm,
            true,
            true,
        )?;
        Ok(unsafe { &mut *Page::as_mut_slice_ptr(page) })
    }

    pub fn as_translation_table(&mut self) -> TranslationTable<'_> {
        TranslationTable::new(&mut self.table, AddressSpace::User)
    }
}

impl Drop for PageTable {
    fn drop(&mut self) {
        self.as_translation_table().unmap_all();
    }
}
