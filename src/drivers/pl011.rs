use crate::{
    alloc::{Arc, Box},
    arch::halt,
    drivers::char_dev::CharDev,
    error::{Error, Result},
    fs::File,
    interrupts::InterruptHandler,
    ring_buffer::RingBuffer,
    sched::{sleep_on, wakeup},
    sys::User,
    volatile_reg_cell, volatile_reg_cell_write, volatile_reg_read, volatile_reg_write, SpinLock,
    Unique,
};
use core::{cell::UnsafeCell, ptr};

#[repr(C)]
pub struct Pl011Regs {
    data: UnsafeCell<u32>,
    status: u32,
    _reserved0: [u32; 4],
    flag: u32,
    _reserved1: u32,
    load_power_counter: u32,
    integer_baud_rate: u32,
    fractional_baud_rate: u32,
    line_control: u32,
    control: u32,
    interrupt_level_select: u32,
    interrupt_mask: UnsafeCell<u32>,
    raw_interrupt_status: u32,
    masked_interrupt_status: u32,
    interrupt_clear: UnsafeCell<u32>,
    dma_control_register: u32,
}

use static_assertions::const_assert;
const_assert!(core::mem::offset_of!(Pl011Regs, flag) == 0x18);
const_assert!(core::mem::offset_of!(Pl011Regs, interrupt_mask) == 0x38);
const_assert!(core::mem::offset_of!(Pl011Regs, interrupt_clear) == 0x44);

const FLAG_RX_FIFO_EMPTY: u32 = 1 << 4;
const FLAG_TX_FIFO_FULL: u32 = 1 << 5;
const INT_CLEAR_RX: u32 = 1 << 4;
const INT_MASK_ALLOW_RX: u32 = 1 << 4;

impl Pl011Regs {
    pub fn data(&self) -> u8 {
        (unsafe { self.data.get().read_volatile() }) as u8
    }
    pub fn set_data(&self, val: u8) {
        unsafe { self.data.get().write_volatile(val as u32) }
    }

    volatile_reg_cell_write!(interrupt_clear);
    volatile_reg_cell!(interrupt_mask);

    volatile_reg_read!(flag);
    volatile_reg_write!(integer_baud_rate);
    volatile_reg_write!(fractional_baud_rate);
}

pub struct Pl011 {
    regs: Unique<Pl011Regs>,
    read_buffer: SpinLock<Box<RingBuffer<128>>>,
}

const CLOCK_DIVISOR: u32 = 16;
const FRACTIONAL_PRECISION: u32 = 64;

impl Pl011 {
    pub fn new(mut regs: Unique<Pl011Regs>, clock: u32, baud: u32) -> Result<Self> {
        let integer_divisor = clock / baud / CLOCK_DIVISOR;
        let remainder = clock - (integer_divisor * baud * CLOCK_DIVISOR);
        let fractional_divisor = remainder * FRACTIONAL_PRECISION;

        regs.set_integer_baud_rate(integer_divisor);
        regs.set_fractional_baud_rate(fractional_divisor);

        let buffer = Box::<RingBuffer<128>>::zeroed()?;
        Ok(Self {
            regs,
            read_buffer: SpinLock::new(buffer),
        })
    }

    pub fn enable_rx(&self) {
        let mask = self.regs.interrupt_mask() | INT_MASK_ALLOW_RX;
        self.regs.set_interrupt_mask(mask);
    }

    pub fn disable_rx(&self) {
        let new_mask = self.regs.interrupt_mask() & !INT_MASK_ALLOW_RX;
        self.regs.set_interrupt_mask(new_mask);
    }

    fn tx_queue_full(&self) -> bool {
        self.regs.flag() & FLAG_TX_FIFO_FULL == FLAG_TX_FIFO_FULL
    }

    fn wait_queue_emtpy(&self) {
        // Loop since we wakeup both on rx and tx
        while self.tx_queue_full() {
            // Try to sleep.
            // This will fail if there is not current process (pre-scheduler initialization), so we
            // execute halt and wait-for-interrupt
            if sleep_on(ptr::from_ref(self).addr()).is_err() {
                halt();
            };
        }
    }
}

struct Pl011File {
    uart: Arc<Pl011>,
}

impl File for Pl011File {
    fn read(&mut self, buf: &mut [User<u8>]) -> Result<()> {
        self.uart.read(buf)
    }

    fn seek(&mut self, _position: crate::fs::SeekFrom) -> Result<()> {
        Err(Error::NotSeekable)
    }
}

impl CharDev for Pl011 {
    fn read(&self, buf: &mut [User<u8>]) -> Result<()> {
        {
            let mut buffer = self.read_buffer.lock();
            for byte in buf.iter_mut() {
                match buffer.pop() {
                    Some(val) => byte.write(val)?,
                    None => break,
                };
            }
        }
        self.enable_rx();
        Ok(())
    }

    fn write(&self, buf: &[User<u8>]) -> Result<()> {
        for byte in buf {
            self.regs.set_data(byte.load()?);
            self.wait_queue_emtpy()
        }
        Ok(())
    }

    fn open(self: Arc<Self>) -> Result<Box<dyn File>> {
        Ok(Box::new(Pl011File { uart: self })?)
    }
}

impl InterruptHandler for Pl011 {
    fn ack_interrupt(&self) {
        let mut buffer = self.read_buffer.lock();
        while self.regs.flag() & FLAG_RX_FIFO_EMPTY == 0 && buffer.free_len() > 0 {
            let val = self.regs.data();
            buffer.push(val);
        }
        if buffer.free_len() == 0 {
            self.disable_rx();
        }
        wakeup(ptr::from_ref(self).addr());
        self.regs.set_interrupt_clear(INT_CLEAR_RX);
    }
}
