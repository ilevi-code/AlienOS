use core::ops::{BitAnd, Not, Sub};

pub trait OverflowingAdd: Sized {
    fn overflowing_add(self, rhs: Self) -> (Self, bool);
}

impl OverflowingAdd for usize {
    fn overflowing_add(self, rhs: Self) -> (usize, bool) {
        self.overflowing_add(rhs)
    }
}

trait Integer {
    fn max() -> Self;

    fn one() -> Self;
}

impl Integer for usize {
    fn max() -> Self {
        usize::MAX
    }

    fn one() -> Self {
        1
    }
}

/// Note: Alignemnts must be a power of two
pub trait Align {
    fn align_down(self, align: Self) -> Self;
    fn align_up(self, align: Self) -> Self;
}

impl<T> Align for T
where
    T: Integer + OverflowingAdd + Copy + Sub<Output = T> + Not<Output = T> + BitAnd<Output = T>,
{
    fn align_down(self, align: Self) -> Self {
        let mask = align - Self::one();
        self & (!mask)
    }

    fn align_up(self, align: Self) -> Self {
        let mask = align - Self::one();
        match self.overflowing_add(align - Self::one()) {
            (value, false) => value & (!mask),
            _ => Self::max(),
        }
    }
}
