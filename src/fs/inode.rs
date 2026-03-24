use static_assertions::const_assert;

#[repr(C)]
pub struct Inode {
    _mode: u16,
    _uid: u16,
    __size: u32,
    _atime: u32,
    _ctime: u32,
    _mtime: u32,
    _dtime: u32,
    _gid: u16,
    _link_count: u16,
    _blocks: u32,
    _flags: u32,
    _osd1: u32,
    pub block: [u32; 15],
    _generation: u32,
    _file_acl: u32,
    _dir_acl: u32,
    _faddr: u32,
    _osd2: [u8; 12],
}

const_assert!(core::mem::size_of::<Inode>() == 128);
