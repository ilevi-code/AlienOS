use core::{cmp::min, slice};

#[cfg(feature = "logging")]
use crate::println;

use crate::{
    alloc::{Arc, Box, Vec},
    drivers::block::SECTOR_SIZE,
    error::{Error, Result},
    fs::{File, FileSystem, Path, SeekFrom},
    interrupts::RegSet,
    log,
    mmu::{PagePerm, PageTable},
    num::{AlignDown, AlignUp},
    sched::with_current,
    sys::{
        AsUserBytes, ElfHeader, ProgramHeader, SyscallResult, User, ELF_IDENT_CLASS32,
        ELF_IDENT_DATA_2LSB, ELF_IDENT_MAGIC, ELF_MACHINE_ARM, ELF_SEGMENT_TYPE_LOAD,
        ELF_TYPE_EXEC, ELF_VERSION_CURRENT,
    },
    syscall,
};

syscall!(exec);

fn exec(regs: &mut RegSet) -> SyscallResult {
    let mut dest = [0_u8; 10];
    crate::sys::copy_from_user(&mut dest, unsafe {
        slice::from_raw_parts(regs.r[1] as *const User<u8>, 10)
    })?;
    let path = Path::new(&dest);

    log!("exec: {path:?}");

    let fs = with_current(|current| Arc::clone(&current.fs))?;
    let file = FileSystem::open(Arc::clone(&fs), path)?;
    exec_load(file, regs)?;
    Ok(0)
}

fn exec_load(mut elf: Box<dyn File>, regs: &mut RegSet) -> Result<()> {
    let mut header = ElfHeader::default();
    elf.read(header.as_user_bytes())?;
    verify_elf_header(&header)?;

    let mut program_headers = Vec::<ProgramHeader>::new();
    program_headers.resize(header.program_header_num as usize, ProgramHeader::default())?;
    elf.seek(SeekFrom::Start(header.program_headers_offset as usize))?;
    elf.read(program_headers[..].as_user_bytes())?;

    let mut new_mappings = PageTable::new()?;
    for program_header in &program_headers {
        if program_header.segment_type == ELF_SEGMENT_TYPE_LOAD {
            if program_header.file_size != program_header.mem_size {
                todo!("support bss");
            }
            map_file(
                &mut *elf,
                program_header.file_offset as usize,
                &mut new_mappings,
                program_header.virt_addr as usize,
                program_header.file_size as usize,
            )?;
        }
    }
    new_mappings.apply_user();
    with_current(|current| current.page_table = new_mappings)?;
    regs.lr = header.entry as usize;

    Ok(())
}

fn verify_elf_header(header: &ElfHeader) -> Result<()> {
    if header.ident.magic != ELF_IDENT_MAGIC
        || header.ident.class != ELF_IDENT_CLASS32
        || header.ident.data != ELF_IDENT_DATA_2LSB
        || header.ident.os_abi != 0
        || header.elf_type != ELF_TYPE_EXEC
        || header.machine != ELF_MACHINE_ARM
        || header.version != ELF_VERSION_CURRENT
        || header.elf_header_size as usize != size_of::<ElfHeader>()
        || header.program_header_entry_size as usize != size_of::<ProgramHeader>()
    {
        with_current(|current| {
            if current.pid == 1 {
                panic!("Handle non executable in init");
            }
        })?;
        Err(Error::BadElf)
    } else {
        Ok(())
    }
}

fn map_file(
    file: &mut dyn File,
    file_offset: usize,
    page_table: &mut PageTable,
    virt: usize,
    len: usize,
) -> Result<()> {
    let file_offset = file_offset.align_down(SECTOR_SIZE);
    let virt = virt.align_down(SECTOR_SIZE);
    let len = len.align_up(SECTOR_SIZE);
    map_file_sector_aligned(file, file_offset, page_table, virt, len)
}

fn map_file_sector_aligned(
    file: &mut dyn File,
    file_offset: usize,
    page_table: &mut PageTable,
    mut virt: usize,
    mut len: usize,
) -> Result<()> {
    file.seek(SeekFrom::Start(file_offset))?;

    while len > 0 {
        let page = page_table.alloc_page(virt, PagePerm::UserRw)?;
        let read_size = min(len, page.len());
        file.read(page[..read_size].as_user_bytes())?;

        virt += read_size;
        len -= read_size;
    }

    Ok(())
}
