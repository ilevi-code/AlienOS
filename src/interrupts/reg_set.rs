#[repr(C)]
#[derive(Default, Clone)]
pub struct RegSet {
    pub r: [usize; 13],
    pub lr: usize,
    pub cpsr: usize,
}
