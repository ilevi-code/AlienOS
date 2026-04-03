use static_assertions::const_assert;

pub const PAGE_SIZE: usize = 4096;

#[repr(align(4096))]
pub struct Page(#[allow(unused)] [u8; PAGE_SIZE]);

impl Page {
    pub fn as_slice_ptr(this: *const Self) -> *const [u8] {
        this as *const [u8; PAGE_SIZE] as *const [u8]
    }

    pub fn as_mut_slice_ptr(this: *mut Self) -> *mut [u8] {
        this as *mut [u8; PAGE_SIZE] as *mut [u8]
    }
}

const_assert!(align_of::<Page>() == PAGE_SIZE);
