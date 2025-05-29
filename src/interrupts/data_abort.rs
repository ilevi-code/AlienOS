fn read_fault_register() -> usize {
    let fault_address: usize;
    unsafe {
        core::arch::asm!("MRC p15, 0, {}, c6, c0, 0", out(reg) fault_address);
    }
    fault_address
}

#[no_mangle]
pub(super) extern "C" fn data_abort_handler(reg_set: *mut RegSet) {
    crate::console::println!(
        "fault acessing address 0x{:x} from 0x{:x}",
        read_fault_register(),
        unsafe { &*reg_set }.lr,
    );
}
