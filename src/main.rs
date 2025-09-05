#![reexport_test_harness_main = "test_main"]
#![feature(
    pointer_is_aligned_to,
    negative_impls,
    custom_test_frameworks,
    format_args_nl
)]
#![test_runner(crate::testing::test_runner)]
#![no_std]
#![no_main]

mod alloc;
mod arch;
mod console;
mod device_tree;
mod drivers;
mod error;
mod gic;
mod heap;
mod interrupts;
mod kernel_location;
mod memory_model;
mod mmu;
mod num;
mod panic_handler;
mod phys;
mod semihosting;
mod spinlock;
mod step_range;
mod testing;

use core::slice;

use alloc::Vec;
use arch::PeMode;
use console::Pl011Regs;
use console::SERIAL;
use device_tree::{DeviceTree, Memory};
use kernel_location::get_kernel_location;
use mmu::TranslationTable;
use spinlock::SpinLock;

use crate::{
    alloc::Unique,
    interrupts::{Interrupt, InterruptController},
};

const KERN_LINK: usize = 0xc000_0000;

#[no_mangle]
#[allow(clippy::missing_safety_doc, unreachable_code)]
pub unsafe extern "C" fn main(dtb: usize, _bootstrap_table: usize, stack_top: usize) -> ! {
    let dtb_address = memory_model::phys_to_virt(&phys::Phys::<u8>::from(dtb));
    let device_tree = DeviceTree::from(dtb_address);

    let memory = device_tree
        .parse_node_type::<Memory>("memory")
        .expect("DeviceTree must contains a \"memory\" node");
    heap::init(
        get_kernel_location().end,
        KERN_LINK + memory.addresses.len(),
    );

    let mut raw_device_tree = Vec::<u8>::new();
    raw_device_tree
        .extend_from_slice(slice::from_raw_parts(
            dtb_address as *const u8,
            device_tree.len(),
        ))
        .expect("Device tree is too big");
    let device_tree = DeviceTree::from(raw_device_tree.as_mut_ptr());

    let root = device_tree.parse_root().expect("Failed to parse DTB");

    interrupts::dup_stack(stack_top).expect("Failed to duplicate stack");
    for pe_mode in [PeMode::Irq, PeMode::Abort] {
        interrupts::setup_interrupt_stacks(pe_mode).expect("Failed to map interrupt stack");
    }

    interrupts::without_irq(|| {
        *interrupts::CONTROLLER.lock() = Some(InterruptController::new(
            TranslationTable::get_kernel()
                .map_device(root.interrupt_controller.distributor)
                .expect("Failed to map interrupt-controller distributor")
                .into(),
            TranslationTable::get_kernel()
                .map_device(root.interrupt_controller.cpu_interface)
                .expect("Failed to map interrupt-controller cpu-interface")
                .into(),
        ));
    });

    #[cfg(test)]
    {
        test_main();
        semihosting::shutdown(0);
    }

    interrupts::CONTROLLER
        .lock()
        .as_mut()
        .unwrap()
        .register(root.pl011.interrupt.interrupt, console_isr);
    let mut uart: Unique<Pl011Regs> = TranslationTable::get_kernel()
        .map_device(root.pl011.address)
        .unwrap()
        .into();
    let mask = uart.interrupt_mask() | 1 << 4;
    uart.set_interrupt_mask(mask);
    *SERIAL.lock() = uart;

    interrupts::without_irq(|| {
        interrupts::CONTROLLER
            .lock()
            .as_mut()
            .unwrap()
            .register(root.timer.virt_timer.interrupt, timer_isr);
    });
    let mut timer = interrupts::VirtualCounter;
    timer.enable();
    timer.arm(timer.frequency());

    let mut kern_table = TranslationTable::get_kernel();
    let disk_mmio = kern_table
        .map_device(phys::Phys::<drivers::virtio_blk::regs::VirtioRegs>::from(
            0xa003e00,
        ))
        .unwrap();
    let blk = drivers::virtio_blk::VirtioBlkBuilder::new(Unique::from(disk_mmio)).unwrap();
    let queue = drivers::virtio_blk::virt_queue::VirtQueue::new().unwrap();
    let mut blk = blk.add_queue(queue).unwrap();
    let mut r = alloc::Box::<drivers::virtio_blk::block::Request>::zeroed().unwrap();
    r.request_type = drivers::virtio_blk::block::VIRTIO_BLK_T_OUT;
    r.data[0] = 1;
    unsafe { core::arch::asm!("CPSID i") };
    blk.write(r);

    let mut lock = DISK.lock();
    *lock = Some(blk);
    drop(lock);
    interrupts::without_irq(|| {
        interrupts::CONTROLLER
            .lock()
            .as_mut()
            .unwrap()
            .register(Interrupt::Spi(0x2f), disk_isr);
    });

    unsafe { core::arch::asm!("CPSIE i") };
    for _ in 0..1000_usize {
        core::hint::black_box(1);
    }
    #[allow(clippy::empty_loop)]
    loop {}
}

fn console_isr(_int_num: u32, _reg_set: &mut interrupts::RegSet) {
    let mut data = [0u32; 4];
    let mut index = 0;

    {
        let mut uart = SERIAL.lock();
        while uart.flag() & (1 << 4) == 0 {
            if index < data.len() {
                data[index] = uart.data();
                index += 1;
            } else {
                uart.data();
            }
        }
        uart.set_interrupt_clear(1 << 4);
    }

    console::println!("console: {data:x?}");
}

fn timer_isr(_int_num: u32, _reg_set: &mut interrupts::RegSet) {
    let mut timer = interrupts::VirtualCounter;
    timer.arm(timer.frequency());
    console::println!("Timer!");
}

fn disk_isr(_int_num: u32, _reg_set: &mut interrupts::RegSet) {
    let mut guard = DISK.lock();
    let Some(blk) = guard.as_mut() else {
        return;
    };
    blk.status();
    blk.interrupt_ack();
    blk.check_used();
}

static DISK: spinlock::SpinLock<Option<drivers::virtio_blk::VirtioBlk>> =
    spinlock::SpinLock::new(None);
