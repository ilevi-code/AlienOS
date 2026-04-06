use core::{mem::offset_of, ptr};

use crate::{
    alloc::{Box, Unique},
    arch::data_sync,
    drivers::{
        block::{Device, SECTOR_SIZE},
        virtio::{
            mmio_regs::VirtioRegs,
            queue::{DesctriptorIndex, Flag, VirtQueue},
        },
    },
    interrupts::{without_irq, InterruptHandler},
    phys::Phys,
    sched::{sleep_on, wakeup},
    spinlock::SpinLock,
};

const VIRTIO_BLK_T_IN: u32 = 0;
const VIRTIO_BLK_T_OUT: u32 = 1;

#[repr(C, packed)]
pub struct Request {
    pub request_type: u32,
    pub reserved: u32,
    pub sector: u64,
    /* 512 bytes of data */
    pub status: u8,
}

struct RequestDescriptors<'a> {
    queue: &'a SpinLock<Box<VirtQueue>>,
    header: DesctriptorIndex,
    data: DesctriptorIndex,
    trailer: DesctriptorIndex,
}

impl<'a> Drop for RequestDescriptors<'a> {
    fn drop(&mut self) {
        let mut queue = self.queue.lock();
        queue.free_descriptor(self.header);
        queue.free_descriptor(self.data);
        queue.free_descriptor(self.trailer);
    }
}

pub struct VirtioBlk {
    regs: Unique<VirtioRegs>,
    queue: SpinLock<Box<VirtQueue>>,
}

impl VirtioBlk {
    pub(super) fn new(regs: Unique<VirtioRegs>, queue: SpinLock<Box<VirtQueue>>) -> Self {
        Self { regs, queue }
    }
    fn sleep_on_descriptor(&self, descriptor: DesctriptorIndex) -> crate::error::Result<()> {
        loop {
            let descriptor_busy = without_irq(|| -> crate::error::Result<bool> {
                sleep_on(core::ptr::from_ref(self).addr())?;
                Ok(self.queue.lock().is_descriptor_busy(descriptor))
            })?;
            if !descriptor_busy {
                break;
            }
        }
        Ok(())
    }

    fn alloc_request_descriptors(&self) -> Option<RequestDescriptors<'_>> {
        let mut queue = self.queue.lock();
        let header = queue.alloc_descriptor()?;
        let Some(data) = queue.alloc_descriptor() else {
            queue.free_descriptor(header);
            return None;
        };
        let Some(trailer) = queue.alloc_descriptor() else {
            queue.free_descriptor(header);
            queue.free_descriptor(data);
            return None;
        };
        Some(RequestDescriptors {
            queue: &self.queue,
            header,
            data,
            trailer,
        })
    }

    fn submit_request(
        &self,
        request: &Request,
        buf: &[u8],
        descriptors: &RequestDescriptors,
        data_flags: u16,
    ) {
        let mut queue = self.queue.lock();

        let phys = Phys::from_virt(ptr::from_ref(request) as *const u8);
        {
            let header_start = queue.descriptor_at(descriptors.header);
            header_start.addr = phys;
            // The last byte is the status, and should be in a writable portion of the request
            header_start.length = size_of::<Request>() as u32 - 1;
            header_start.flags = Flag::Next as u16;
            header_start.next = descriptors.data;
        }

        {
            let data_descriptor = queue.descriptor_at(descriptors.data);
            data_descriptor.addr = Phys::from_virt(buf.as_ptr());
            data_descriptor.length = 512;
            data_descriptor.flags = (Flag::Next as u16) | data_flags;
            data_descriptor.next = descriptors.trailer;
        }

        {
            let header_status = queue.descriptor_at(descriptors.trailer);
            header_status.addr = unsafe { phys.byte_add(offset_of!(Request, status)) };
            header_status.length = size_of::<u8>() as u32;
            header_status.flags = Flag::Write as u16;
            header_status.next = DesctriptorIndex::UNUSED;
        }

        queue.submit(descriptors.header);
    }
}

impl Device for VirtioBlk {
    fn read(&self, buf: &mut [u8; SECTOR_SIZE], sector: usize) -> crate::error::Result<()> {
        let request = Request {
            request_type: VIRTIO_BLK_T_IN,
            reserved: 0,
            sector: sector as u64,
            status: 0,
        };

        let descriptors = loop {
            if let Some(descriptors) = self.alloc_request_descriptors() {
                break descriptors;
            }
        };

        self.submit_request(&request, buf, &descriptors, Flag::Write as u16);

        data_sync();

        without_irq(|| {
            // always using queue #0
            unsafe { self.regs.queue_notify.get().write_volatile(0) };
        });

        self.sleep_on_descriptor(descriptors.header)?;

        Ok(())
    }

    fn write(&self, buf: &[u8; SECTOR_SIZE], sector: usize) -> crate::error::Result<()> {
        let request = Request {
            request_type: VIRTIO_BLK_T_OUT,
            reserved: 0,
            sector: sector as u64,
            status: 0,
        };

        let descriptors = loop {
            if let Some(descriptors) = self.alloc_request_descriptors() {
                break descriptors;
            }
        };

        self.submit_request(&request, buf, &descriptors, 0);

        data_sync();

        without_irq(|| {
            // always using queue #0
            unsafe { self.regs.queue_notify.get().write_volatile(0) };
        });

        self.sleep_on_descriptor(descriptors.header)?;

        Ok(())
    }
}

impl InterruptHandler for VirtioBlk {
    fn ack_interrupt(&self) {
        let mut queue = self.queue.lock();
        queue.check_used_ring_progress();
        let int_status = self.regs.interrupt_status();
        // Safety:
        // regs are MMIO
        unsafe { self.regs.interrupt_ack.get().write_volatile(int_status) };
        wakeup(ptr::from_ref(self).addr());
    }
}
