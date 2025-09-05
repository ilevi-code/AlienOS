use core::ops::Sub;

use crate::error::{Error, Result};

// Offset an on address in the current table
#[derive(Clone, Copy)]
pub(super) struct Offset(pub(super) usize);

impl Sub<Offset> for Offset {
    type Output = usize;

    fn sub(self, rhs: Offset) -> Self::Output {
        self.0 - rhs.0
    }
}

pub(super) struct AddrParts {
    addr: usize,
}

impl AddrParts {
    pub(super) fn section_offset(&self) -> usize {
        (self.l2_index() << 12) + self.page_offset()
    }

    #[inline]
    pub(super) fn l1_index(&self) -> usize {
        self.addr >> 20
    }

    #[inline]
    pub(super) fn l2_index(&self) -> usize {
        (self.addr >> 12) & 0xff
    }

    #[inline]
    pub(super) fn page_offset(&self) -> usize {
        self.addr & 0xfff
    }

    pub(super) fn try_add(&mut self, bytes: usize) -> Result<()> {
        self.addr += bytes;
        if self.addr >= 0x8000_0000 {
            Err(Error::OutOfRange)
        } else {
            Ok(())
        }
    }

    #[inline]
    pub(super) fn addr(&self) -> usize {
        self.addr
    }
}

impl From<Offset> for AddrParts {
    fn from(offset: Offset) -> Self {
        Self { addr: offset.0 }
    }
}
