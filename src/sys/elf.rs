#[derive(Default, Debug)]
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

#[derive(Default, Debug)]
#[repr(C)]
pub struct ElfHeader {
    pub ident: ElfIdentifier,
    pub elf_type: u16,
    pub machine: u16,
    pub version: u32,
    entry: u32,
    program_headers_offset: u32,
    section_headers_offset: u32,
    flags: u32,
    ehsize: u16,
    program_header_entry_size: u16,
    program_header_num: u16,
    section_header_entry_size: u16,
    section_header_num: u16,
    section_hreader_string_index: u16,
}

static_assertions::const_assert!(core::mem::size_of::<ElfHeader>() == 52);

pub const ELF_IDENT_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
pub const ELF_IDENT_CLASS32: u8 = 1;
pub const ELF_IDENT_DATA_2LSB: u8 = 1;
pub const ELF_TYPE_EXEC: u16 = 2;
pub const ELF_MACHINE_ARM: u16 = 40;
pub const ELF_VERSION_CURRENT: u32 = 1;
