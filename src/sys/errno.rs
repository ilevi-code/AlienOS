use crate::error::Error;

pub enum Errno {
    Fault,
    InvalidFd,
    NoSyscall,
    OutOfMem,
    NoSuchDevice,
    Internal,
}

impl From<crate::error::Error> for Errno {
    fn from(value: crate::error::Error) -> Self {
        match value {
            Error::OutOfMem => Errno::OutOfMem,
            Error::MemoryFault => Errno::Fault,
            Error::NoDevice => Errno::NoSuchDevice,
            Error::PerCpuReborrow
            | Error::NoCurrentProcess
            | Error::Unsupproted
            | Error::OutOfRange
            | Error::Remap
            | Error::LayoutError(_) => Errno::Internal,
        }
    }
}

pub type SyscallResult = core::result::Result<usize, Errno>;
