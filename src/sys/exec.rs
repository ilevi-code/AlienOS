use core::slice;

use crate::{
    alloc::{Arc, Box, Vec},
    error::{Error, Result},
    fs::{File, FileSystem, Path, SeekFrom},
    interrupts::RegSet,
    println,
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
    println!("exec: {path:?}");
    let fs = with_current(|current| Arc::clone(&current.fs))?;
    let file = FileSystem::open(Arc::clone(&fs), path)?;
    exec_load(file)?;
    Ok(0)
}

fn exec_load(mut elf: Box<dyn File>) -> Result<()> {
    let mut header = ElfHeader::default();
    elf.read(header.as_user_bytes())?;
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
        return Err(Error::BadElf);
    }
    let mut program_headers = Vec::<ProgramHeader>::new();
    program_headers.resize(header.program_header_num as usize, ProgramHeader::default())?;
    elf.seek(SeekFrom::Start(header.program_headers_offset as usize))?;
    elf.read(program_headers[..].as_user_bytes())?;
    for program_header in &program_headers {
        if program_header.segment_type == ELF_SEGMENT_TYPE_LOAD {
            println!("{:x?}", program_header);
        }
    }
    todo!();
}
