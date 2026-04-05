pub const BIT_PER_U32: usize = 32;

pub trait Bitmap {
    fn set(&mut self, bit_index: usize);
    fn unset(&mut self, bit_index: usize);
    fn is_set(&self, bit_index: usize) -> bool;
}

impl Bitmap for [u32] {
    fn set(&mut self, bit_index: usize) {
        let index = bit_index / BIT_PER_U32;
        let shift = bit_index % BIT_PER_U32;
        self[index] |= 1 << shift;
    }

    fn unset(&mut self, bit_index: usize) {
        let index = bit_index / BIT_PER_U32;
        let shift = bit_index % BIT_PER_U32;
        self[index] &= !(1 << shift);
    }

    fn is_set(&self, bit_index: usize) -> bool {
        let index = bit_index / BIT_PER_U32;
        let shift = bit_index % BIT_PER_U32;
        let mask = 1 << shift;
        self[index] & mask == mask
    }
}
