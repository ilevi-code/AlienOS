// TODO rename to error
#[derive(Debug)]
pub enum MapError {
    Remap,
    AllocError,
}

pub(super) type Result<T> = core::result::Result<T, MapError>;
