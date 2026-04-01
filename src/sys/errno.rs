use crate::error::Error;

pub enum Errno {
    Fault,
    InvalidFd,
    NoSyscall,
    OutOfMem,
    NoSuchDevice,
    Internal,
    NotADir,
    NoEntry,
    IsADir,
    NotExecutable,
}

impl From<crate::error::Error> for Errno {
    fn from(value: crate::error::Error) -> Self {
        match value {
            Error::BadElf => Errno::NotExecutable,
            Error::OutOfMem => Errno::OutOfMem,
            Error::MemoryFault => Errno::Fault,
            Error::NoDevice => Errno::NoSuchDevice,
            Error::IsADir => Errno::IsADir,
            Error::PerCpuReborrow
            | Error::NoCurrentProcess
            | Error::Unsupproted
            | Error::OutOfRange
            | Error::Remap
            | Error::BadDowncast
            | Error::LayoutError(_) => Errno::Internal,
            Error::NoEntry => Errno::NotADir,
            Error::NotADir => Errno::NoEntry,
        }
    }
}

pub type SyscallResult = core::result::Result<usize, Errno>;
