mod block;
mod kern_allocator;
mod kern_heap;

pub use kern_heap::init;
pub(crate) use kern_heap::ALLOCATOR;
pub use kern_heap::{alloc, dealloc};
