use alien_derive::IntEnum;
use static_assertions::const_assert;

#[derive(IntEnum, Debug)]
pub enum FileType {
    Regular = 0x8000,
    Directory = 0x4000,
    CharDev = 0x2000,
    #[default]
    Unknown,
}

#[repr(C)]
pub struct Inode {
    mode: u16,
    _uid: u16,
    pub size: u32,
    _atime: u32,
    _ctime: u32,
    _mtime: u32,
    _dtime: u32,
    _gid: u16,
    _link_count: u16,
    pub blocks: u32,
    _flags: u32,
    _osd1: u32,
    pub block: [u32; 15],
    _generation: u32,
    _file_acl: u32,
    _dir_acl: u32,
    _faddr: u32,
    _osd2: [u8; 12],
}

impl Inode {
    pub fn file_type(&self) -> FileType {
        (self.raw_file_type() as u32).into()
    }

    fn raw_file_type(&self) -> u16 {
        const FILE_TYPE_MASK: u16 = 0xf000;
        self.mode & FILE_TYPE_MASK
    }
}

const_assert!(core::mem::size_of::<Inode>() == 128);
