enum InterruptType {
    Spi,
    Ppi,
}

struct Interrupt {
    int_type: InterruptType,
    number: u32,
    flags: u16,
}

struct IterruptIterator<'a> {
    slice: &'a [u8],
}

impl<'a> Iterator for IterruptIterator<'a> {
    fn next
}
