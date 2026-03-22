use atomic_enum::atomic_enum;

use crate::{
    alloc::{Arc, Box, Vec},
    error::{Error, Result},
    mmu::PAGE_SIZE,
    spinlock::SpinLock,
    sys::Errno,
};

pub struct User<T: ?Sized>(T);

pub trait File {
    fn read(&mut self, buf: User<[u8]>) -> core::result::Result<(), Errno>;
}

use core::{marker::PhantomData, ptr::null_mut, sync::atomic::AtomicUsize};

#[atomic_enum]
#[derive(PartialEq)]
pub enum State {
    Sleeping,
    Runnable,
    Running,
    Zombie,
}

pub struct PageTable(pub usize);

#[repr(align(4096))]
pub struct KernelStack(#[allow(unused)] pub [u8; PAGE_SIZE]);

type FdTable = SpinLock<Vec<Option<SpinLock<Box<dyn File>>>>>;

pub struct Process {
    pub pid: u32,
    pub page_table: PageTable,
    pub kern_stack: Box<KernelStack>,
    pub sp: *mut u8,
    #[allow(unused)]
    pub fd: FdTable,
    pub chan: AtomicUsize,
    pub state: AtomicState,
}

pub struct StackPointer<'stack> {
    base: *mut u8,
    size: usize,
    _phantom: PhantomData<&'stack u8>,
}

impl<'stack> StackPointer<'stack> {
    pub fn from_slice(s: &'stack mut [u8]) -> Self {
        Self {
            base: unsafe { s.as_mut_ptr().add(s.len()) },
            size: s.len(),
            _phantom: PhantomData,
        }
    }
    pub fn alloc_frame<Frame>(&mut self) -> Result<&mut Frame> {
        let mut size = size_of::<Frame>();
        let add_to_align = self.base.align_offset(align_of::<Frame>());
        if add_to_align != 0 {
            size += size_of::<Frame>() - add_to_align;
        }
        match self.size.checked_sub(size) {
            Some(new_size) => {
                // Guarenteed by creation the pointer has at least `size` consequitive bytes, which
                // means this will not overflow.
                self.base = unsafe { self.base.sub(size) };
                let frame = self.base as *mut Frame;
                self.size = new_size;
                // We do not hangle alignment
                Ok(unsafe { &mut *frame })
            }
            None => Err(Error::OutOfMem),
        }
    }

    pub fn into_sp(self) -> *mut u8 {
        self.base
    }
}

impl Process {
    pub fn with_pid(pid: u32) -> Result<Self> {
        Ok(Self {
            pid,
            page_table: PageTable(0),
            kern_stack: Box::<KernelStack>::zeroed()?,
            sp: null_mut(),
            fd: SpinLock::new(Vec::new()),
            chan: AtomicUsize::new(0),
            state: AtomicState::new(State::Runnable),
        })
    }
}
