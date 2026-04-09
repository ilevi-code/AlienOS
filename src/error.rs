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
    #[error("Memory fault")]
    MemoryFault,
    #[error("Unsupported")]
    Unsupproted,
    #[error("No current process")]
    NoCurrentProcess,
    #[error("No such device")]
    NoDevice,
    #[error("Entry not found")]
    NoEntry,
    #[error("Not a directory")]
    NotADir,
    #[error("Is a directory")]
    IsADir,
    #[error("Bad ELF")]
    BadElf,
    #[error("Invalid offset")]
    InvalidOffset,
    #[error("Not seekable")]
    NotSeekable,
    #[error("Bad file type")]
    BadFileType,
    #[error("End of syscall args")]
    EndOfSyscallArgs,
    #[error("Name too long")]
    NameTooLong,
    #[error("Bad file descriptor")]
    BadFd,
}

pub(super) type Result<T> = core::result::Result<T, Error>;
