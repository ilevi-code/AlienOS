pub unsafe fn write(s: &str) {
    let uart0 = 0x9000000 as *mut u8;
    for byte in s.bytes() {
        *uart0 = byte;
    }
}
