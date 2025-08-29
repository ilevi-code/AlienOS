use core::{
    ptr::NonNull,
    sync::atomic::{AtomicPtr, Ordering},
};

use crate::{SpinLock, Unique};

#[repr(C)]
pub struct Pl011Regs {
    data: u32,
    status: u32,
    _reserved0: [u32; 4],
    flag: u32,
    _reserved1: u32,
    load_power_counter: u32,
    integer_buad_rate: u32,
    fractional_buad_rate: u32,
    line_control: u32,
    control: u32,
    interrupt_level_select: u32,
    interrupt_mask: u32,
    raw_interrupt_status: u32,
    masked_interrupt_status: u32,
    interrupt_clear: u32,
    dma_control_register: u32,
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

use static_assertions::const_assert;
const_assert!(core::mem::offset_of!(Pl011Regs, flag) == 0x18);
const_assert!(core::mem::offset_of!(Pl011Regs, interrupt_mask) == 0x38);
const_assert!(core::mem::offset_of!(Pl011Regs, interrupt_clear) == 0x44);

use core::ptr::{addr_of, addr_of_mut};
use paste::paste;

impl Pl011Regs {
    pub fn reset(&mut self) {
        self.status = 0;
    }

    volatile_reg!(data);
    volatile_reg_read!(flag);
    volatile_reg!(interrupt_mask);
    volatile_reg_write!(interrupt_clear);
}

// pub static serial: SpinLock<Unique<Pl011Regs>> = AtomicPtr::new(as *mut u8);
pub static SERIAL: SpinLock<Unique<Pl011Regs>> = SpinLock::new(Unique::from_non_null(
    NonNull::new(0x9000000 as *mut Pl011Regs).unwrap(),
));
