use core::{arch::asm, mem::offset_of, ptr::NonNull};

use crate::{
    alloc::{Box, Unique},
    drivers::block::Device,
    phys::{Phys, PhysMut},
    spinlock::SpinLock,
};

#[derive(Debug)]
pub enum AllocError {
    OutOfMem,
}
#[inline]
fn data_sync() {
    unsafe { asm!("dsb") }
}
pub mod virt_queue {
    use core::ptr::addr_of;

    use crate::{alloc::Box, phys::Phys};

    pub enum Flag {
        Next = 1,
        Write = 2,
        Indirect = 4,
    }
    pub const VIRT_QUEUE_SIZE: usize = 128;

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
    }

    #[repr(C, packed)]
    #[derive(Clone, Copy)]
    struct UsedElement {
        id: DesctriptorIndex,
        len: u32,
    }

    #[repr(C, align(16))]
    pub struct VirtQueue {
        descriptors: [Descriptor; VIRT_QUEUE_SIZE],
        available: AvailableRing,
        used: UsedRing,
        free_desc: DesctriptorIndex,
    }

    use static_assertions::const_assert;

    use super::{data_sync, AllocError};
    const_assert!(core::mem::offset_of!(VirtQueue, descriptors) % 16 == 0);
    const_assert!(core::mem::offset_of!(VirtQueue, available) % 2 == 0);
    const_assert!(core::mem::offset_of!(VirtQueue, used) % 4 == 0);
    const_assert!(core::mem::align_of::<VirtQueue>() == 16);
    const_assert!(core::mem::size_of::<VirtQueue>() < 0x4096);

    #[derive(Clone, Copy)]
    pub struct DesctriptorIndex(u16);

    impl DesctriptorIndex {
        pub const LAST: DesctriptorIndex = DesctriptorIndex(0);
    }

    impl VirtQueue {
        pub fn new() -> Result<Box<VirtQueue>, AllocError> {
            let Ok(mut queue) = Box::<VirtQueue>::zeroed() else {
                return Err(AllocError::OutOfMem);
            };
            for (i, descriptor) in queue.descriptors.iter_mut().enumerate() {
                descriptor.next = DesctriptorIndex(((i + 1) % VIRT_QUEUE_SIZE) as u16);
            }
            Ok(queue)
        }

        pub fn alloc_descriptor(&mut self) -> DesctriptorIndex {
            let desc = self.free_desc;
            self.free_desc = self.descriptors[self.free_desc.0 as usize].next;
            desc
        }

        pub fn descriptor_at(&mut self, index: DesctriptorIndex) -> &mut Descriptor {
            &mut self.descriptors[index.0 as usize]
        }

        pub fn submit(&mut self, index: DesctriptorIndex) {
            crate::console::println!("submit idx: {:x}", index.0);
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

        pub fn used_index(&self) -> DesctriptorIndex {
            let i = self.used.index;
            self.used.ring[i as usize].id
        }
    }
}

macro_rules! volatile_reg_read {
    ($field:tt) => {
        #[inline]
        pub fn $field(&self) -> u32 {
            unsafe { addr_of!(self.$field).read_volatile() }
        }
    };
}

macro_rules! volatile_reg_write {
    ($field:tt) => {
        paste! {
            #[inline]
            pub fn [< set_ $field >] (&mut self, value: u32) {
                unsafe { addr_of_mut!(self.$field).write_volatile(value) }
            }
        }
    };
}

macro_rules! volatile_reg {
    ($field:tt) => {
        volatile_reg_read!($field);
        volatile_reg_write!($field);
    };
}

enum DeviceId {
    Network = 1,
    Block = 2,
}

const VIRIO_MAGIC: u32 = 0x74726976;
const VIRTIO_VERSION: u32 = 2;
const DEVICE_STATUS_ACK: u32 = 1;
const DEVICE_STATUS_DRIVER: u32 = 2;
const DEVICE_STATUS_DRIVER_OK: u32 = 4;
const DEVICE_STATUS_FEATURES_OK: u32 = 8;
pub mod regs {
    use core::{
        cell::UnsafeCell,
        ptr::{addr_of, addr_of_mut},
    };
    use paste::paste;

    #[repr(C)]
    pub struct VirtioRegs {
        magic: u32,
        version: u32,
        device_id: u32,
        vendor_id: u32,
        device_features: u32,
        device_features_sel: u32,
        _reserved0: [u32; 2],
        driver_features: u32,
        driver_features_sel: u32,
        _reserved1: [u32; 2],
        queue_sel: u32,
        queue_num_max: u32,
        queue_num: u32,
        _reserved2: [u32; 2],
        queue_ready: u32,
        _reserved3: [u32; 2],
        pub queue_notify: UnsafeCell<u32>,
        _reserved4: [u32; 3],
        interrupt_status: u32,
        pub interrupt_ack: UnsafeCell<u32>,
        _reserved5: [u32; 2],
        status: u32,
        _reserved6: [u32; 3],
        queue_desc_low: u32,
        queue_desc_high: u32,
        _reserved7: [u32; 2],
        queue_avail_low: u32,
        queue_avail_high: u32,
        _reserved8: [u32; 2],
        queue_used_low: u32,
        queue_used_high: u32,
        _reserved9: [u32; 21],
        config_generation: u32,
    }

    use static_assertions::const_assert;
    const_assert!(core::mem::offset_of!(VirtioRegs, queue_notify) == 0x50);

    impl VirtioRegs {
        fn config_mut<Config>(&mut self) -> &mut Config {
            let addr = self as *mut Self;
            let config_ptr = unsafe { addr.add(1) } as *mut Config;
            unsafe { &mut *config_ptr }
        }

        pub fn reset(&mut self) {
            self.status = 0;
        }

        volatile_reg_read!(magic);
        volatile_reg_read!(version);
        volatile_reg_read!(device_id);
        volatile_reg!(status);
        volatile_reg_write!(device_features_sel);
        volatile_reg_write!(driver_features_sel);

        volatile_reg_write!(queue_sel);
        volatile_reg_read!(queue_num_max);
        volatile_reg_write!(queue_num);
        volatile_reg!(queue_ready);
        // volatile_reg_read!(queue_notify);

        volatile_reg_read!(interrupt_status);
        // volatile_reg!(interrupt_ack);

        volatile_reg_write!(queue_desc_low);
        volatile_reg_write!(queue_desc_high);
        volatile_reg_write!(queue_avail_low);
        volatile_reg_write!(queue_avail_high);
        volatile_reg_write!(queue_used_low);
        volatile_reg_write!(queue_used_high);
    }
}

pub mod block {
    use core::ptr::{addr_of, addr_of_mut};
    use paste::paste;

    #[repr(C, packed)]
    struct Geometry {
        cylinders: u16,
        heads: u8,
        sectros: u8,
    }

    #[repr(C, packed)]
    struct Topology {
        physical_block_exp: u8,
        alignment_offset: u8,
        min_io_size: u16,
        opt_io_size: u32,
    }
    #[repr(C, packed)]
    pub struct VirtioBlkConfig {
        capacity_low: u32,
        capacity_high: u32,
        size_max: u32,
        seg_max: u32,
        geometry: Geometry,
        blk_size: u32,
        toplogy: Topology,
        writeback: u8,
        _unused0: u8,
        num_queues: u16,
        max_discard_sectors: u32,
        max_discard_seg: u32,
        discard_sector_alignemtn: u32,
        max_write_zeroed_sectors: u32,
        max_write_zeroed_seg: u32,
        write_zeroes_may_unmap: u8,
        _unused1: [u8; 3],
        max_secure_erase_sectors: u32,
        max_secure_erase_seg: u32,
        secure_erase_sector_alignment: u32,
    }

    impl VirtioBlkConfig {
        volatile_reg_read!(capacity_low);
    }

    const VIRTIO_BLK_T_IN: u32 = 0;
    pub const VIRTIO_BLK_T_OUT: u32 = 1;
    #[repr(C, packed)]
    pub struct Request {
        pub request_type: u32,
        pub reserved: u32,
        pub sector: u64,
        /* 512 bytes of data */
        pub status: u8,
    }
}

#[derive(Debug)]
pub enum Error {
    BadMagic,
    BadVersion,
    FeatureNegotiationFailed,
    AllocError,
    QueueInUse,
    QueueTooBig,
    QueueUnavailable,
}

pub struct VirtioBlkBuilder {
    regs: Unique<regs::VirtioRegs>,
}

pub struct VirtioBlk {
    regs: Unique<regs::VirtioRegs>,
    queue: SpinLock<Box<virt_queue::VirtQueue>>,
}

impl VirtioBlkBuilder {
    pub fn new(mut regs: Unique<regs::VirtioRegs>) -> Result<Self, Error> {
        crate::console::println!("{:?}", regs.as_ptr());
        if regs.magic() != VIRIO_MAGIC {
            return Err(Error::BadMagic);
        }
        if regs.version() != VIRTIO_VERSION {
            return Err(Error::BadVersion);
        }
        assert!(regs.device_id() == 2);

        regs.reset();
        data_sync();

        let mut status = regs.status() | DEVICE_STATUS_ACK;
        regs.set_status(status);
        data_sync();

        status |= DEVICE_STATUS_DRIVER;
        regs.set_status(status);
        data_sync();

        regs.set_device_features_sel(0);
        regs.set_driver_features_sel(0);

        status |= DEVICE_STATUS_FEATURES_OK;
        regs.set_status(status);
        data_sync();
        if regs.status() & DEVICE_STATUS_FEATURES_OK == 0 {
            return Err(Error::FeatureNegotiationFailed);
        }

        let config = unsafe {
            regs.as_ptr()
                .byte_add(0x100)
                .cast::<block::VirtioBlkConfig>()
        };
        let config = unsafe { config.as_ref() }.unwrap();
        crate::console::println!("status: {:x}", regs.status());
        crate::console::println!("disk contains {:x} sectors", config.capacity_low());

        // fill the blk-config
        status |= DEVICE_STATUS_DRIVER_OK;
        regs.set_status(status);

        while regs.status() & DEVICE_STATUS_DRIVER_OK == 0 {}

        Ok(Self { regs })
    }

    pub fn add_queue(mut self, queue: Box<virt_queue::VirtQueue>) -> Result<VirtioBlk, Error> {
        self.regs.set_queue_sel(0);
        data_sync();
        if self.regs.queue_ready() != 0 {
            return Err(Error::QueueInUse);
        }
        let max_queue_size = self.regs.queue_num_max() as usize;
        if max_queue_size == 0 {
            return Err(Error::QueueUnavailable);
        }
        if max_queue_size < virt_queue::VIRT_QUEUE_SIZE {
            return Err(Error::QueueTooBig);
        }
        self.regs.set_queue_num(virt_queue::VIRT_QUEUE_SIZE as u32);
        let (descriptor_address, avaiable_address, used_address) = queue.areas();
        self.regs.set_queue_desc_low(descriptor_address as u32);
        self.regs.set_queue_desc_high(0);
        self.regs.set_queue_avail_low(avaiable_address as u32);
        self.regs.set_queue_avail_high(0);
        self.regs.set_queue_used_low(used_address as u32);
        self.regs.set_queue_used_high(0);
        self.regs.set_queue_ready(1);

        Ok(VirtioBlk {
            regs: self.regs,
            queue: SpinLock::new(queue),
        })
    }
}

impl VirtioBlk {
    pub fn status(&self) {
        crate::console::println!("status: {:x}", self.regs.status());
        crate::console::println!("int status: {:x}", self.regs.interrupt_status());
        self.check_used()
    }

    pub fn check_used(&self) {
        let mut queue = self.queue.lock();
        let i = queue.used_index();
        let descriptor = queue.descriptor_at(i);
        if descriptor.flags & (virt_queue::Flag::Next as u16) == 0 {
            crate::console::println!("bad descriptor");
        }
        let next = descriptor.next;
        let descriptor = queue.descriptor_at(next);
        let addr = descriptor.addr;
        let addr = addr.into_virt();
        crate::console::println!("desc status: {:x}", unsafe { addr.read_volatile() });
    }
}

impl Device for VirtioBlk {
    fn read(
        &self,
        buf: &mut [u8; super::block::SECTOR_SIZE],
        sector: usize,
    ) -> crate::error::Result<()> {
        todo!()
    }

    fn write(
        &self,
        buf: &[u8; super::block::SECTOR_SIZE],
        sector: usize,
    ) -> crate::error::Result<()> {
        let request = block::Request {
            request_type: block::VIRTIO_BLK_T_OUT,
            reserved: 0,
            sector: sector as u64,
            status: 0,
        };
        let mut queue = self.queue.lock();
        let index = queue.alloc_descriptor();
        let index2 = queue.alloc_descriptor();
        let index3 = queue.alloc_descriptor();

        let phys = Phys::from_virt(&raw const request as *const u8);
        {
            let header_start = queue.descriptor_at(index);
            header_start.addr = phys;
            header_start.length = size_of::<block::Request>() as u32 - 1;
            header_start.flags = virt_queue::Flag::Next as u16;
            header_start.next = index2;
        }

        {
            let data_descriptor = queue.descriptor_at(index2);
            data_descriptor.addr = Phys::from_virt(buf.as_ptr());
            // let aaa = Phys::from_virt(buf.as_ptr()).addr();
            // crate::println!("AAAAAAAAAAAA {:?} {:x}", buf.as_ptr(), aaa);
            data_descriptor.length = 512;
            data_descriptor.flags = virt_queue::Flag::Next as u16;
            data_descriptor.next = index3;
        }

        {
            let header_status = queue.descriptor_at(index3);
            header_status.addr = unsafe { phys.byte_add(offset_of!(block::Request, status)) };
            header_status.length = size_of::<u8>() as u32;
            header_status.flags = virt_queue::Flag::Write as u16;
            header_status.next = virt_queue::DesctriptorIndex::LAST;
        }

        queue.submit(index);
        drop(queue);
        data_sync();

        // always using queue #0
        unsafe { self.regs.queue_notify.get().write_volatile(0) };

        Ok(())
    }

    fn ack_interrupt(&self) {
        self.status();
        let int_status = self.regs.interrupt_status();
        // SAFETY:
        // regs are MMIO
        unsafe { self.regs.interrupt_ack.get().write_volatile(int_status) };
    }
}
