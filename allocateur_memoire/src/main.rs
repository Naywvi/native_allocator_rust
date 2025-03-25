mod allocator;

use std::alloc::{alloc, Layout};
use std::mem;

fn test_allocator() {
    println!("\n------ * ------");
    let a = Box::new(42u32);
    println!("a = {}, address = {:p}, size = {} octets",a,a,mem::size_of_val(&*a));

    let b = Box::new([0u8; 128]);
    println!("b = [u8; 128], address = {:p}, size = {} octets",b.as_ptr(),mem::size_of_val(&*b));

    let c = Box::new("hello rust");
    println!("c = {}, address = {:p}, size = {} octets",c,c.as_ptr(),mem::size_of_val(&*c));
    println!("\nMémoire utilisée : {} / {} octets",allocator::ALLOCATOR.allocated_bytes(),allocator::ALLOCATOR.heap_size());

    // Allocation manuelle
    let layout = Layout::from_size_align(64 * 1024, 8).unwrap();

    unsafe {
        let ptr = alloc(layout);
        if ptr.is_null() {
            println!("✅ Allocation échouée comme prévu (plus assez de mémoire)");
        } else {
            println!("❌ ATTENTION : allocation réussie alors qu’elle ne devrait pas");
        }
    }
    println!("Mémoire après tentative : {} / {} octets",allocator::ALLOCATOR.allocated_bytes(),allocator::ALLOCATOR.heap_size());
    println!("\n------ * ------");
}

fn main() {
    test_allocator();
}