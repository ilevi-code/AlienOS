use crate::{
    alloc::{Box, Unique},
    arch::data_sync,
    drivers::virtio::{
        block_dev_regs::VirtioBlkConfig, queue::VIRT_QUEUE_SIZE, VirtQueue, VirtioBlk, VirtioRegs,
    },
    spinlock::SpinLock,
};

const VIRIO_MAGIC: u32 = 0x74726976;
const VIRTIO_VERSION: u32 = 2;
const DEVICE_STATUS_ACK: u32 = 1;
const DEVICE_STATUS_DRIVER: u32 = 2;
const DEVICE_STATUS_DRIVER_OK: u32 = 4;
const DEVICE_STATUS_FEATURES_OK: u32 = 8;

enum DeviceId {
    Block = 2,
}

#[derive(Debug)]
pub enum Error {
    BadMagic,
    BadVersion,
    FeatureNegotiationFailed,
    QueueInUse,
    QueueTooBig,
    QueueUnavailable,
}

pub struct VirtioBlkBuilder {
    regs: Unique<VirtioRegs>,
}

impl VirtioBlkBuilder {
    pub fn new(mut regs: Unique<VirtioRegs>) -> Result<Self, Error> {
        if regs.magic() != VIRIO_MAGIC {
            return Err(Error::BadMagic);
        }
        if regs.version() != VIRTIO_VERSION {
            return Err(Error::BadVersion);
        }
        debug_assert!(regs.device_id() == DeviceId::Block as u32);

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

        let config = regs.config_mut::<VirtioBlkConfig>();
        crate::console::println!("disk contains {:x} sectors", config.capacity_low());

        // fill the blk-config
        status |= DEVICE_STATUS_DRIVER_OK;
        regs.set_status(status);

        while regs.status() & DEVICE_STATUS_DRIVER_OK == 0 {}

        Ok(Self { regs })
    }

    pub fn add_queue(mut self, queue: Box<VirtQueue>) -> Result<VirtioBlk, Error> {
        self.regs.set_queue_sel(0);
        data_sync();
        if self.regs.queue_ready() != 0 {
            return Err(Error::QueueInUse);
        }
        let max_queue_size = self.regs.queue_num_max() as usize;
        if max_queue_size == 0 {
            return Err(Error::QueueUnavailable);
        }
        if max_queue_size < VIRT_QUEUE_SIZE {
            return Err(Error::QueueTooBig);
        }
        self.regs.set_queue_num(VIRT_QUEUE_SIZE as u32);
        let (descriptor_address, avaiable_address, used_address) = queue.areas();
        self.regs.set_queue_desc_low(descriptor_address as u32);
        self.regs.set_queue_desc_high(0);
        self.regs.set_queue_avail_low(avaiable_address as u32);
        self.regs.set_queue_avail_high(0);
        self.regs.set_queue_used_low(used_address as u32);
        self.regs.set_queue_used_high(0);
        self.regs.set_queue_ready(1);

        Ok(VirtioBlk::new(self.regs, SpinLock::new(queue)))
    }
}
