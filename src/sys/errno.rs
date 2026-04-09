use crate::error::Error;

pub enum Errno {
    Fault,
    NoSyscall,
    OutOfMem,
    NoSuchDevice,
    Internal,
    NotADir,
    NoEntry,
    IsADir,
    NotExecutable,
    InvalidOffset,
    NotSeekable,
    BadFileType,
    NameTooLong,
    BadFd,
}

impl From<crate::error::Error> for Errno {
    fn from(value: crate::error::Error) -> Self {
        match value {
            Error::BadFd => Errno::BadFd,
            Error::BadFileType => Errno::BadFileType,
            Error::InvalidOffset => Errno::InvalidOffset,
            Error::BadElf => Errno::NotExecutable,
            Error::OutOfMem => Errno::OutOfMem,
            Error::MemoryFault => Errno::Fault,
            Error::NoDevice => Errno::NoSuchDevice,
            Error::IsADir => Errno::IsADir,
            Error::NotSeekable => Errno::NotSeekable,
            Error::NameTooLong => Errno::NameTooLong,
            Error::NoCurrentProcess
            | Error::Unsupproted
            | Error::OutOfRange
            | Error::Remap
            | Error::EndOfSyscallArgs
            | Error::LayoutError(_) => Errno::Internal,
            Error::NoEntry => Errno::NotADir,
            Error::NotADir => Errno::NoEntry,
        }
    }
}

pub type SyscallResult = core::result::Result<usize, Errno>;
