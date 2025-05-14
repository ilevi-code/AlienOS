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

    fn align_up_overflowing(self, align: Self) -> (Self, bool)
    where
        Self: Sized;
}

impl<T> AlignUp for T
where
    T: Integer
        + Rem<Output = T>
        + Sub<Output = T>
        + Add<Output = T>
        + PartialEq
        + Copy
        + OverflowingAdd,
{
    fn align_up(self, align: Self) -> Self {
        let rem = self % align;
        if rem == Self::zero() {
            self
        } else {
            let (value, overflow) = (self - rem).overflowing_add(align);
            if overflow {
                Self::max()
            } else {
                value
            }
        }
    }

    fn align_up_overflowing(self, align: Self) -> (Self, bool)
    where
        Self: Sized,
    {
        let rem = self % align;
        if rem == Self::zero() {
            (self, false)
        } else {
            (self - rem).overflowing_add(align)
        }
    }
}
