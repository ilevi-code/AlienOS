use core::alloc::LayoutError;

use thiserror_no_std::Error;

#[derive(Error, Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub(crate) enum Error {
    #[error("{0}")]
    LayoutError(#[from] LayoutError),
    #[error("out of memory")]
    OutOfMem,
    #[error("remap")]
    Remap,
    #[error("Address out of range")]
    OutOfRange,
}

pub(super) type Result<T> = core::result::Result<T, Error>;
