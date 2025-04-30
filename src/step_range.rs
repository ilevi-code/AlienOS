use core::cmp::Ord;
use core::mem;
use core::ops::{Add, Range};

use crate::num::{AlignDown, AlignUp, OverflowingAdd};

#[derive(Clone, Debug)]
pub struct StepRange<T> {
    pub start: Option<T>,
    pub end: T,
    pub step: T,
}

impl<T> StepRange<T> {
    pub fn new(start: T, end: T, step: T) -> Self {
        Self {
            start: Some(start),
            end: end,
            step,
        }
    }
}

impl<T: AlignUp + AlignDown + Copy> StepRange<T> {
    pub fn align_from(range: Range<T>, step: T) -> Self {
        let end = range.end.align_down(step);
        Self {
            start: Some(range.start.align_down(step)),
            end: end,
            step,
        }
    }
}

impl<T: Add<Output = T> + Ord + Copy + OverflowingAdd> Iterator for StepRange<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(start) = self.start else {
            return None;
        };
        if start > self.end {
            return None;
        }
        let (n, overflow) = start.overflowing_add(self.step);
        if overflow {
            return mem::replace(&mut self.start, None);
        }
        mem::replace(&mut self.start, Some(n))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn normal_range() {
        let mut range = StepRange::align_from(0..5, 2);
        assert_eq!(range.next(), Some(0));
        assert_eq!(range.next(), Some(2));
        assert_eq!(range.next(), Some(4));
        assert_eq!(range.next(), None);
    }

    #[test_case]
    fn overflowing_range() {
        let mut range = StepRange::align_from(0xffff_ffe0..0xffff_fff1, 0x10);
        assert_eq!(range.next(), Some(0xffff_ffe0));
        assert_eq!(range.next(), Some(0xffff_fff0));
        assert_eq!(range.next(), None);
    }
}
