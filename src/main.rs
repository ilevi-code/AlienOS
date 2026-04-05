#![reexport_test_harness_main = "test_main"]
#![feature(
    pointer_is_aligned_to,
    negative_impls,
    custom_test_frameworks,
    coerce_unsized,
    unsize,
    slice_index_methods,
    dispatch_from_dyn,
    arbitrary_self_types
)]
#![test_runner(crate::testing::test_runner)]
#![no_std]
#![no_main]

mod alloc;
mod arch;
mod bitmap;
mod console;
mod device_tree;
mod drivers;
mod entry;
mod error;
mod fs;
mod gic;
mod heap;
mod interrupts;
mod memory_model;
mod mmu;
mod num;
mod panic_handler;
mod phys;
mod sched;
mod semihosting;
mod spinlock;
mod step_range;
mod sys;
mod testing;

use core::slice;

use alloc::Vec;
use arch::PeMode;
use console::Pl011Regs;
use console::SERIAL;
use device_tree::{DeviceTree, Memory};
use memory_model::{get_kernel_location, KERN_LINK};
use mmu::TranslationTable;
use spinlock::SpinLock;

use crate::alloc::Arc;
use crate::interrupts::Interrupt;
use crate::sys::register_disk;
use crate::{alloc::Unique, interrupts::InterruptController};

#[no_mangle]
#[allow(clippy::missing_safety_doc, unreachable_code)]
pub unsafe extern "C" fn main(dtb: usize, _bootstrap_table: usize) -> ! {
    let dtb = phys::PhysMut::<u8>::from(dtb).into_virt();
    let device_tree = DeviceTree::from(dtb);

    let memory = device_tree
        .parse_node_type::<Memory>("memory")
        .expect("DeviceTree must contains a \"memory\" node");
    heap::init(
        get_kernel_location().end,
        KERN_LINK + memory.addresses.len(),
    );

    let mut raw_device_tree = Vec::<u8>::new();
    raw_device_tree
        .extend_from_slice(slice::from_raw_parts(dtb, device_tree.len()))
        .expect("Device tree is too big");
    let device_tree = DeviceTree::from(raw_device_tree.as_mut_ptr());

    let root = device_tree.parse_root().expect("Failed to parse DTB");

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

    interrupts::without_irq(|| {
        interrupts::CONTROLLER
            .lock()
            .as_mut()
            .unwrap()
            .register(root.pl011.interrupt.interrupt, console_isr);
    });
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
    let blk = blk.add_queue(queue).unwrap();
    let disk = Arc::new(blk).expect("Failed to allocation disk struct");
    register_disk(
        Arc::<drivers::virtio_blk::VirtioBlk>::clone(&disk),
        Interrupt::Spi(0x2f),
    )
    .expect("Failed to register disk");

    sys::init_syscalls().expect("Failed to init syscall table");
    sched::setup_init_proc().expect("Failed to setup init");
    let e = interrupts::call_in_new_stack(run_scheduler);
    panic!("Failed to call scheduler in new stack {:?}", e);
}

extern "C" fn run_scheduler() -> ! {
    sched::sched()
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
