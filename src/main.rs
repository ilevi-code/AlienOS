#![reexport_test_harness_main = "test_main"]
#![feature(pointer_is_aligned_to, negative_impls, custom_test_frameworks)]
#![test_runner(crate::testing::test_runner)]
#![no_std]
#![no_main]

mod arch;
mod console;
mod device_tree;
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
    let device_tree = device_tree::DeviceTree::from(dtb_address);

    let memory = device_tree
        .parse_node_type::<device_tree::Memory>("memory")
        .expect("DeviceTree should contains \"memory\" node");
    heap::init(
        get_kernel_location().end,
        KERN_LINK + memory.addresses.len(),
    );

    init_mmu_fine_grained();

    interrupts::init_interrupt_handler();

    loop {}
}

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
