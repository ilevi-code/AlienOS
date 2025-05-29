#[repr(C)]
#[derive(Default, Clone)]
pub(super) struct RegSet {
    r: [usize; 13],
    lr: usize,
    cpsr: usize,
}
