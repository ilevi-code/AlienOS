use core::cmp::Ord;
use core::mem;
use core::ops::Add;

#[derive(Clone)]
pub struct StepRange<T> {
    start: T,
    end: T,
    step: T,
}

impl<T> StepRange<T> {
    pub fn new(start: T, end: T, step: T) -> Self {
        Self { start, end, step }
    }
}

impl<T: Add<Output = T> + Ord + Clone> Iterator for StepRange<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start < self.end {
            let n = self.start.clone() + self.step.clone();
            Some(mem::replace(&mut self.start, n))
        } else {
            None
        }
    }
}
