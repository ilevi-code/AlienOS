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
mod mmio;
mod mmu;
mod num;
mod panic_handler;
mod phys;
mod ring_buffer;
mod sched;
mod semihosting;
mod spinlock;
mod step_range;
mod sys;
mod testing;

use core::slice;

use alloc::Vec;
use arch::PeMode;
use console::SERIAL;
use device_tree::{DeviceTree, Memory};
use drivers::pl011::Pl011Regs;
use memory_model::{get_kernel_location, KERN_LINK};
use mmu::TranslationTable;
use spinlock::SpinLock;

use crate::alloc::Arc;
use crate::drivers::char_dev::{Major, char_dev_register};
use crate::drivers::pl011::Pl011;
use crate::interrupts::{register_handler, Interrupt};
use crate::sys::register_disk;
use crate::{alloc::Unique, interrupts::InterruptController};

// Just because, no real reason
const SERIAL_BAUD_RATE: u32 = 115200;

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

    let uart: Unique<Pl011Regs> = TranslationTable::get_kernel()
        .map_device(root.pl011.address)
        .unwrap()
        .into();
    let uart = Arc::new(Pl011::new(uart, root.clock.frequency, SERIAL_BAUD_RATE).unwrap()).unwrap();
    *SERIAL.lock() = Some(Arc::clone(&uart));
    register_handler(Arc::<Pl011>::clone(&uart), root.pl011.interrupt.interrupt).unwrap();
    uart.enable_rx();
    char_dev_register(uart, Major::Pl011).expect("Failed to register uart as chardev");

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
            .register(root.timer.virt_timer.interrupt, timer_isr);
    });
    let mut timer = interrupts::VirtualCounter;
    timer.enable();
    timer.arm(timer.frequency());

    let mut kern_table = TranslationTable::get_kernel();
    // TODO get address from DTB
    let disk_mmio = kern_table
        .map_device(phys::Phys::<drivers::virtio::VirtioRegs>::from(0xa003e00))
        .expect("Mapping disk failed");
    let blk = drivers::virtio::VirtioBlkBuilder::new(Unique::from(disk_mmio))
        .expect("Hardware negotiation failed");
    let queue = drivers::virtio::VirtQueue::new().expect("virt-queue allocation failed");
    let blk = blk.add_queue(queue).expect("Queue negotiation failed");
    let disk = Arc::new(blk).expect("Failed to allocation disk struct");
    register_disk(Arc::<drivers::virtio::VirtioBlk>::clone(&disk))
        .expect("Failed to register disk");
    register_handler(
        Arc::<drivers::virtio::VirtioBlk>::clone(&disk),
        Interrupt::Spi(0x2f),
    )
    .expect("Failed to register disk as handler");

    sys::init_syscalls().expect("Failed to init syscall table");
    sched::setup_init_proc().expect("Failed to setup init");
    let e = interrupts::call_in_new_stack(run_scheduler);
    panic!("Failed to call scheduler in new stack {:?}", e);
}

extern "C" fn run_scheduler() -> ! {
    sched::sched()
}

fn timer_isr(_int_num: u32, _reg_set: &mut interrupts::RegSet) {
    let mut timer = interrupts::VirtualCounter;
    timer.arm(timer.frequency());
    console::println!("Timer!");
}
