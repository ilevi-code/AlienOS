use core::cmp::Ord;
use core::mem;
use core::ops::{Add, Range};

use crate::num::{Align, OverflowingAdd};

#[derive(Clone)]
pub struct StepRange<T> {
    pub start: T,
    pub end: T,
    pub step: T,
}

impl<T> StepRange<T> {
    pub fn new(start: T, end: T, step: T) -> Self {
        Self { start, end, step }
    }
}

impl<T: Align + Copy> StepRange<T> {
    pub fn align_from(range: Range<T>, step: T) -> Self {
        Self {
            start: range.start.align_down(step),
            end: range.end.align_up(step),
            step,
        }
    }
}

impl<T: Add<Output = T> + Ord + Copy + OverflowingAdd> Iterator for StepRange<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start < self.end {
            return None;
        }
        let (n, overflow) = self.start.overflowing_add(self.step);
        if overflow {
            return None;
        }
        Some(mem::replace(&mut self.start, n))
    }
}
