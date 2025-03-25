use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let heap_start = HEAP.0.as_ptr() as usize;
        let heap_end = heap_start + HEAP_SIZE;

        let mut current = self.next.load(Ordering::Relaxed);

        loop {
            let alloc_start = Self::align_up(heap_start + current, layout.align());
            let alloc_end = alloc_start + layout.size();

            if alloc_end > heap_end {
                return null_mut();
            }

            let next_offset = alloc_end - heap_start;

            match self.next.compare_exchange(
                current,
                next_offset,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => return alloc_start as *mut u8,
                Err(old) => current = old,
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[global_allocator]
pub static ALLOCATOR: BumpAllocator = BumpAllocator::new();
