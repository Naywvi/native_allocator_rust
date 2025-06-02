// Import des traits et types nécessaires à la gestion de la mémoire bas-niveau
use core::alloc::{GlobalAlloc, Layout};          // Trait GlobalAlloc + Layout de blocs mémoire
use core::ptr::null_mut;                         // Pour retourner un pointeur nul si échec d'allocation
use core::sync::atomic::{AtomicUsize, Ordering}; // Permet une allocation thread-safe via des opérations atomiques

// Taille totale du heap en octets : ici, 64 Ko
const HEAP_SIZE: usize = 64 * 1024;

// Structure représentant notre heap statique, aligné sur 8 octets
// Le #[repr(align(N))] garantit un alignement mémoire pour les architectures modernes
#[repr(align(8))]
struct AlignedHeap([u8; HEAP_SIZE]);

// Allocation statique de notre buffer mémoire (le "heap" custom)
// L'utilisation de "unsafe" ici est nécessaire car les accès à une zone statique mutable peuvent entraîner des conditions de course si mal utilisés.
// → https://doc.rust-lang.org/reference/items/static-items.html#mutable-statics
static mut HEAP: AlignedHeap = AlignedHeap([0; HEAP_SIZE]);

// Allocateur bump : alloue de la mémoire de manière séquentielle. Il gère un seul pointeur (next) qui avance dans le heap au fur et à mesure des allocations
// → https://www.youtube.com/watch?v=TZ5a3gCCZYo

// Avantage : très rapide et simple
// Inconvénient : pas de libération de mémoire (pas de free)
pub struct BumpAllocator {
    next: AtomicUsize,
}

impl BumpAllocator {
    // Constructeur de l'allocateur, initialise 'next' à 0 (début du heap)
    pub const fn new() -> Self {
        BumpAllocator {
            next: AtomicUsize::new(0),
        }
    }

    // Aligner une adresse vers le haut (alignement mémoire requis)
    fn align_up(addr: usize, align: usize) -> usize {
        (addr + align - 1) & !(align - 1)
    }

    // Permet de connaître la quantité de mémoire déjà allouée
    pub fn allocated_bytes(&self) -> usize {
        self.next.load(Ordering::Relaxed)
    }

    // Retourne la taille totale disponible sur le heap
    pub fn heap_size(&self) -> usize {
        HEAP_SIZE
    }
}

// Implémentation du trait GlobalAlloc de Rust. Ce trait permet me d'utiliser notre allocateur comme allocateur GLOBAL
// Rust appellera automatiquement `alloc()` et `dealloc()` via `Box`, `Vec`, etc.
unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Définition des bornes de la zone mémoire
        let heap_start = HEAP.0.as_ptr() as usize;
        let heap_end = heap_start + HEAP_SIZE;

        // Récupère la position actuelle dans le heap
        let mut current = self.next.load(Ordering::Relaxed);

        loop {
            // Calcul de la position alignée pour cette allocation
            let alloc_start = Self::align_up(heap_start + current, layout.align());
            let alloc_end = alloc_start + layout.size();

            // Vérifie qu'on ne dépasse pas la taille du heap
            if alloc_end > heap_end {
                return null_mut();// Échec : plus de mémoire disponible
            }

            // Calcule le nouvel offset dans le heap
            let next_offset = alloc_end - heap_start;

            // compare_exchange permet d'assurer que deux threads n'allouent pas le même espace
            match self.next.compare_exchange(
                current,
                next_offset,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ){
                Ok(_) => return alloc_start as *mut u8, // Validé : retourne le pointeur vers la zone allouée
                Err(old) => current = old,              // Échec : quelqu'un d'autre a alloué entre temps, on recommence
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Pas de free
    }
}

// Ce bloc indique que notre allocateur personnalisé devient L'ALLOCATEUR GLOBAL. Toutes les allocations effectuées dans le programme passeront par ce bump allocator.
// → https://doc.rust-lang.org/std/alloc/index.html
#[global_allocator]
pub static ALLOCATOR: BumpAllocator = BumpAllocator::new();