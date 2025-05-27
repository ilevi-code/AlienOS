#![reexport_test_harness_main = "test_main"]
#![feature(pointer_is_aligned_to, negative_impls, custom_test_frameworks)]
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
use device_tree::{DeviceTree, Memory};
use kernel_location::get_kernel_location;
use mmu::TranslationTable;

const KERN_LINK: usize = 0xc000_0000;

#[no_mangle]
#[allow(clippy::missing_safety_doc, unreachable_code)]
pub unsafe extern "C" fn main(dtb: usize, _bootstrap_table: usize) -> ! {
    #[cfg(test)]
    {
        test_main();
        semihosting::shutdown(0);
    }

    let dtb_address = memory_model::phys_to_virt(&phys::Phys::<u8>::from(dtb));
    let device_tree = DeviceTree::from(dtb_address);

    let memory = device_tree
        .parse_node_type::<Memory>("memory")
        .expect("DeviceTree should contains \"memory\" node");
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

    init_mmu_fine_grained();

    let root = device_tree.parse_root();
    console::println!("{:x?}", root);

    interrupts::init_interrupt_handler();

    let mut kern_table = TranslationTable::get_kernel();
    let disk_mmio = kern_table
        .map_device(phys::Phys::<drivers::virtio_blk::regs::VirtioRegs>::from(
            0xa003e00,
        ))
        .unwrap();
    let blk = drivers::virtio_blk::VirtioBlkBuilder::new(drivers::virtio_blk::Unique::from(
        core::ptr::NonNull::new(disk_mmio).unwrap(),
    ))
    .unwrap();
    let queue = drivers::virtio_blk::virt_queue::VirtQueue::new().unwrap();
    let mut blk = blk.add_queue(queue).unwrap();
    let mut r = alloc::Box::<drivers::virtio_blk::block::Request>::zeroed().unwrap();
    r.request_type = 1; // VIRTIO_BLK_T_OUT
    r.data[0] = 1;
    unsafe { core::arch::asm!("CPSID i") };
    blk.write(r);

    let mut lock = DISK.lock();
    *lock = Some(blk);
    drop(lock);
    {
        *interrupts::disk_handler.lock() = Some(disk_isr);
    }
    unsafe { core::arch::asm!("CPSIE i") };

    for _ in 0..1000_usize {
        core::hint::black_box(1);
    }
    // blk.status();
    loop {}
}

fn disk_isr() {
    let mut guard = DISK.lock();
    let Some(blk) = guard.as_mut() else {
        return;
    };
    blk.status();
    blk.interrupt_ack();
    blk.check_used();
    semihosting::shutdown(0);
}

static DISK: spinlock::SpinLock<Option<drivers::virtio_blk::VirtioBlk>> =
    spinlock::SpinLock::new(None);

fn init_mmu_fine_grained() {
    let _kern_location = get_kernel_location();
    let mut kern_table = TranslationTable::get_kernel();
    kern_table.unmap(memory_model::DEVICE_VIRT..0xffef_ffff); // should unmap until 0xffff_ffff, but
                                                              // it used for interrupt stack

    let new_uart = kern_table
        .map_device(phys::Phys::<u8>::from(
            console::UART.load(core::sync::atomic::Ordering::Relaxed) as usize,
        ))
        .unwrap();
    console::println!("new uart at {:?}", new_uart);
    console::UART.store(new_uart, core::sync::atomic::Ordering::Relaxed);

    let new_gicc = kern_table
        .map_device(phys::Phys::<gic::Gicc>::from(
            gic::GICC.load(core::sync::atomic::Ordering::Relaxed) as usize,
        ))
        .unwrap();
    console::println!("new gicc at {:?}", new_uart);
    gic::GICC.store(new_gicc, core::sync::atomic::Ordering::Relaxed);

    let new_gicd = kern_table
        .map_device(phys::Phys::<gic::Gicd>::from(
            gic::GICD.load(core::sync::atomic::Ordering::Relaxed) as usize,
        ))
        .unwrap();
    console::println!("new gicd at {:?}", new_uart);
    gic::GICD.store(new_gicd, core::sync::atomic::Ordering::Relaxed);
}
