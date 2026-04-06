use core::ptr::addr_of;

use crate::{alloc::Box, arch::data_sync, bitmap::Bitmap, phys::Phys};

pub enum Flag {
    Next = 1,
    Write = 2,
}
pub const VIRT_QUEUE_SIZE: usize = 128;
pub const BIT_PER_U32: usize = 32;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Descriptor {
    pub addr: Phys<u8>,
    pub addr_high: u32,
    pub length: u32,
    pub flags: u16,
    pub next: DesctriptorIndex, // 0 for last
}

#[repr(C, packed)]
pub struct AvailableRing {
    flags: u16,
    index: u16,
    ring: [DesctriptorIndex; VIRT_QUEUE_SIZE],
}

#[repr(C, packed)]
struct UsedRing {
    flags: u16,
    index: u16,
    ring: [UsedElement; VIRT_QUEUE_SIZE],
    _avail_event: u16,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct UsedElement {
    id: u32,
    len: u32,
}

#[repr(C, align(16))]
pub struct VirtQueue {
    descriptors: [Descriptor; VIRT_QUEUE_SIZE],
    available: AvailableRing,
    used: UsedRing,
    free_desc: DesctriptorIndex,
    used_bitmap: [u32; VIRT_QUEUE_SIZE / BIT_PER_U32],
    last_seen_used: u16,
}

use static_assertions::const_assert;

#[derive(Debug)]
pub enum AllocError {
    OutOfMem,
}

const_assert!(core::mem::offset_of!(VirtQueue, descriptors) % 16 == 0);
const_assert!(core::mem::offset_of!(VirtQueue, available) % 2 == 0);
const_assert!(core::mem::offset_of!(VirtQueue, used) % 4 == 0);
const_assert!(core::mem::align_of::<VirtQueue>() == 16);
const_assert!(core::mem::size_of::<VirtQueue>() < 0x4096);

#[derive(Clone, Copy, PartialEq)]
pub struct DesctriptorIndex(u16);

impl DesctriptorIndex {
    pub const UNUSED: DesctriptorIndex = DesctriptorIndex(0);
}

impl VirtQueue {
    pub fn new() -> Result<Box<VirtQueue>, AllocError> {
        let Ok(mut queue) = Box::<VirtQueue>::zeroed() else {
            return Err(AllocError::OutOfMem);
        };
        for (i, descriptor) in queue.descriptors.iter_mut().enumerate().skip(1) {
            descriptor.next = DesctriptorIndex(((i + 1) % VIRT_QUEUE_SIZE) as u16);
        }
        // 0 is unused
        queue.free_desc = DesctriptorIndex(1);
        Ok(queue)
    }

    pub fn alloc_descriptor(&mut self) -> Option<DesctriptorIndex> {
        let desc = self.free_desc;
        if desc == DesctriptorIndex::UNUSED {
            return None;
        }
        self.free_desc = self.descriptors[self.free_desc.0 as usize].next;
        self.descriptors[desc.0 as usize].next = DesctriptorIndex(0);
        debug_assert!(!self.used_bitmap.is_set(desc.0 as usize));
        self.used_bitmap.set(desc.0 as usize);
        Some(desc)
    }

    pub fn descriptor_at(&mut self, index: DesctriptorIndex) -> &mut Descriptor {
        &mut self.descriptors[index.0 as usize]
    }

    pub fn submit(&mut self, index: DesctriptorIndex) {
        self.available.ring[self.available.index as usize] = index;
        data_sync();
        self.available.index = self.available.index.wrapping_add(1);
        data_sync();
    }

    pub fn areas(&self) -> (usize, usize, usize) {
        (
            Phys::from_virt(addr_of!(self.descriptors)).addr(),
            Phys::from_virt(addr_of!(self.available)).addr(),
            Phys::from_virt(addr_of!(self.used)).addr(),
        )
    }

    pub fn is_descriptor_busy(&self, descriptor: DesctriptorIndex) -> bool {
        let index = descriptor.0 as usize;
        self.used_bitmap.is_set(index)
    }

    /// Note: used as in consumed, not "current in use"
    pub fn check_used_ring_progress(&mut self) {
        let used_index = unsafe { (&raw mut self.used.index).read_volatile() };
        while self.last_seen_used != used_index {
            let used_descriptor_id =
                self.used.ring[self.last_seen_used as usize % self.used.ring.len()].id;
            debug_assert!(self.used_bitmap.is_set(used_descriptor_id as usize));
            self.used_bitmap.unset(used_descriptor_id as usize);
            self.last_seen_used = self.last_seen_used.wrapping_add(1);
        }
    }

    pub fn free_descriptor(&mut self, descriptor: DesctriptorIndex) {
        debug_assert!(descriptor != DesctriptorIndex::UNUSED);
        self.used_bitmap.unset(descriptor.0 as usize);
        self.descriptors[descriptor.0 as usize].next = self.free_desc;
        self.free_desc = descriptor
    }
}
