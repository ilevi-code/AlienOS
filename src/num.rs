use core::ops::{Add, Rem, Sub};

pub trait OverflowingAdd: Sized {
    fn overflowing_add(self, rhs: Self) -> (Self, bool);
}

impl OverflowingAdd for usize {
    fn overflowing_add(self, rhs: Self) -> (usize, bool) {
        self.overflowing_add(rhs)
    }
}

trait Integer {
    fn zero() -> Self;

    fn max() -> Self;
}

impl Integer for usize {
    fn zero() -> Self {
        0
    }

    fn max() -> Self {
        usize::MAX
    }
}

pub trait AlignDown {
    fn align_down(self, align: Self) -> Self;
}

impl<T> AlignDown for T
where
    T: Integer + Sub<Output = T> + Rem<Output = T> + Copy,
{
    fn align_down(self, align: Self) -> Self {
        self - (self % align)
    }
}

pub trait AlignUp {
    fn align_up(self, align: Self) -> Self;
}

impl<T> AlignUp for T
where
    T: Integer
        + Rem<Output = T>
        + Sub<Output = T>
        + Add<Output = T>
        + PartialEq
        + Copy
        + OverflowingAdd
        + core::fmt::LowerHex,
{
    fn align_up(self, align: Self) -> Self {
        let rem = self % align;
        if rem == Self::zero() {
            self
        } else {
            // (self - rem).checked_add(align).value_or(0xffffffff)
            let (value, overflow) = (self - rem).overflowing_add(align);
            if overflow {
                Self::max()
            } else {
                value
            }
        }
    }
}
