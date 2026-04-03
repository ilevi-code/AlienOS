use alien_derive::Pod;
use alien_traits::Pod;

#[derive(Default, Debug, Pod)]
#[repr(C)]
pub struct ElfIdentifier {
    pub magic: [u8; 4],
    pub class: u8,
    pub data: u8,
    pub version: u8,
    pub os_abi: u8,
    _abi_version: u8,
    _padding: [u8; 7],
}

#[derive(Default, Debug, Pod)]
#[repr(C)]
pub struct ElfHeader {
    pub ident: ElfIdentifier,
    pub elf_type: u16,
    pub machine: u16,
    pub version: u32,
    pub entry: u32,
    pub program_headers_offset: u32,
    section_headers_offset: u32,
    flags: u32,
    pub elf_header_size: u16,
    pub program_header_entry_size: u16,
    pub program_header_num: u16,
    section_header_entry_size: u16,
    section_header_num: u16,
    section_hreader_string_index: u16,
}

static_assertions::const_assert!(core::mem::size_of::<ElfHeader>() == 52);

#[derive(Default, Debug, Clone, Pod)]
#[repr(C)]
pub struct ProgramHeader {
    pub segment_type: u32,
    pub file_offset: u32,
    pub virt_addr: u32,
    _phys_addr: u32,
    pub file_size: u32,
    pub mem_size: u32,
    flags: u32,
    align: u32,
}

pub const ELF_IDENT_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
pub const ELF_IDENT_CLASS32: u8 = 1;
pub const ELF_IDENT_DATA_2LSB: u8 = 1;
pub const ELF_TYPE_EXEC: u16 = 2;
pub const ELF_MACHINE_ARM: u16 = 40;
pub const ELF_VERSION_CURRENT: u32 = 1;

pub const ELF_SEGMENT_TYPE_LOAD: u32 = 1;
