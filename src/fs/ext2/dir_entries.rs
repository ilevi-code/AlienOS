use core::{ptr, slice};

pub struct DirEntries<'a> {
    dir_data: &'a [u8],
    pos: usize,
    block_size: usize,
}

impl<'a> DirEntries<'a> {
    pub fn new(dir_data: &'a [u8], block_size: usize) -> Self {
        Self {
            dir_data,
            pos: 0,
            block_size,
        }
    }
}

pub struct DirEntry<'a> {
    pub inode: u32,
    pub file_type: u8,
    pub name: &'a [u8],
}

#[repr(C, packed)]
struct RawDirEntry {
    inode: u32,
    record_length: u16,
    name_length: u8,
    file_type: u8,
}

impl<'a> Iterator for DirEntries<'a> {
    type Item = DirEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos == self.block_size {
            return None;
        }
        // Safety:
        // `self.pos` is contained within the block, and thus is a valid reference
        let entry_ptr = unsafe {
            ptr::from_ref(self.dir_data)
                .cast::<RawDirEntry>()
                .byte_add(self.pos)
        };
        // Safety:
        // TODO
        let raw_entry = unsafe { entry_ptr.as_ref_unchecked() };
        self.pos += raw_entry.record_length as usize;
        let entry = DirEntry {
            inode: raw_entry.inode,
            file_type: raw_entry.file_type,
            // Safety:
            // TODO
            name: unsafe {
                slice::from_raw_parts(
                    entry_ptr.add(1).cast::<u8>(),
                    raw_entry.name_length as usize,
                )
            },
        };
        Some(entry)
    }
}
