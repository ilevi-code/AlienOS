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
}

impl From<crate::error::Error> for Errno {
    fn from(value: crate::error::Error) -> Self {
        match value {
            Error::InvalidOffset => Errno::InvalidOffset,
            Error::BadElf => Errno::NotExecutable,
            Error::OutOfMem => Errno::OutOfMem,
            Error::MemoryFault => Errno::Fault,
            Error::NoDevice => Errno::NoSuchDevice,
            Error::IsADir => Errno::IsADir,
            Error::NotSeekable => Errno::NotSeekable,
            Error::NoCurrentProcess
            | Error::Unsupproted
            | Error::OutOfRange
            | Error::Remap
            | Error::LayoutError(_) => Errno::Internal,
            Error::NoEntry => Errno::NotADir,
            Error::NotADir => Errno::NoEntry,
        }
    }
}

pub type SyscallResult = core::result::Result<usize, Errno>;
