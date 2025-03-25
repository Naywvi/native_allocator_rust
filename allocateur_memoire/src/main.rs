use core::sync::atomic::{AtomicUsize, Ordering};

const HEAP_SIZE: usize = 64 * 1024;

pub struct BumpAllocator {
    next: AtomicUsize,
}

impl BumpAllocator {
    pub const fn new() -> Self {
        BumpAllocator {
            next: AtomicUsize::new(0),
        }
    }
}
