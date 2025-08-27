#[derive(Copy, Clone, Debug)]
pub enum Interrupt {
    Spi(u8),
    Ppi(u8),
}
