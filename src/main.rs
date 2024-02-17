#![no_std]
#![no_main]

mod panic_handler;
mod console;

#[no_mangle]
pub unsafe extern "C" fn main() -> ! {
    console::write("hello\n");
    panic!("kernel has reached it's end");
}
