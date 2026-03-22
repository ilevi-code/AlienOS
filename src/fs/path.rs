use core::fmt::Debug;

pub struct Path {
    pub bytes: [u8],
}

impl Path {
    pub fn new(bytes: &[u8]) -> &Self {
        unsafe { &*(bytes as *const [u8] as *const Path) }
    }
}

impl Debug for Path {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(str::from_utf8(&self.bytes).unwrap_or("<bad path>"))
    }
}
