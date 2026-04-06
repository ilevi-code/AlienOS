use core::cell::UnsafeCell;

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
    queue_notify: UnsafeCell<u32>,
    _reserved4: [u32; 3],
    interrupt_status: u32,
    interrupt_ack: UnsafeCell<u32>,
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

use crate::{volatile_reg, volatile_reg_cell_write, volatile_reg_read, volatile_reg_write};
const_assert!(core::mem::offset_of!(VirtioRegs, queue_notify) == 0x50);
const_assert!(core::mem::size_of::<VirtioRegs>() == 0x100);

impl VirtioRegs {
    pub fn config_mut<Config>(&mut self) -> &mut Config {
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
    volatile_reg_cell_write!(queue_notify);

    volatile_reg_read!(interrupt_status);
    volatile_reg_cell_write!(interrupt_ack);

    volatile_reg_write!(queue_desc_low);
    volatile_reg_write!(queue_desc_high);
    volatile_reg_write!(queue_avail_low);
    volatile_reg_write!(queue_avail_high);
    volatile_reg_write!(queue_used_low);
    volatile_reg_write!(queue_used_high);
}
