#[derive(Clone, Copy)]
pub enum PagePerm {
    NoOne,
    KernOnly,
    UserRo,
    #[allow(unused)]
    UserRw,
}

impl PagePerm {
    #[inline]
    pub(super) fn translate(&self) -> usize {
        match self {
            PagePerm::NoOne => 0,
            PagePerm::KernOnly => 1,
            PagePerm::UserRo => 2,
            PagePerm::UserRw => 3,
        }
    }
}
