pub trait Pod {}

impl Pod for usize {}
impl Pod for u32 {}
impl Pod for u16 {}
impl Pod for u8 {}

impl<T: Pod, const N: usize> Pod for [T; N] {}
impl<T: Pod> Pod for [T] {}
