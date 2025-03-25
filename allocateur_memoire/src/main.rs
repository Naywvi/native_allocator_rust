use core::sync::atomic::{AtomicUsize, Ordering};

const HEAP_SIZE: usize = 64 * 1024;

#[repr(align(8))]
struct AlignedHeap([u8; HEAP_SIZE]);

static mut HEAP: AlignedHeap = AlignedHeap([0; HEAP_SIZE]);

pub struct BumpAllocator {
    next: AtomicUsize,
}

impl BumpAllocator {
    pub const fn new() -> Self {
        BumpAllocator {
            next: AtomicUsize::new(0),
        }
    }
    fn align_up(addr: usize, align: usize) -> usize {
        (addr + align - 1) & !(align - 1)
    }
    pub fn allocated_bytes(&self) -> usize {
        self.next.load(Ordering::Relaxed)
    }
    pub fn heap_size(&self) -> usize {
        HEAP_SIZE
    }
}
