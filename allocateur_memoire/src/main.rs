// main.rs - Point d'entrée du projet FAT32 avec allocateur personnalisé
//
// Projet : Développement d'un allocateur filesystem FAT32
// Objectif : Montrer qu'on peut faire un système de fichiers sans malloc() standard
//
// Architecture du projet :
// ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
// │   main.rs       │───▶│  allocator.rs   │───▶│   fat32.rs      │
// │ (interface)     │    │ (gestion mem)   │    │ (filesystem)    │
// └─────────────────┘    └─────────────────┘    └─────────────────┘
//
// Flow : main utilise fat32, fat32 utilise automatiquement notre allocateur

mod allocator;          // Notre allocateur bump personnalisé (le cœur du projet)
mod fat32;              // Le système de fichiers FAT32 qu'on a implémenté

use std::alloc::{alloc, Layout};
use std::mem;
use std::io::{self, Write};    // Pour le terminal interactif (flush du stdout)
use fat32::{Fat32FileSystem, FileInfo};

// Storage simulé pour notre "disque dur" FAT32 (10MB)
// Dans un vrai OS, ça serait un vrai disque dur ou une partition
// Ici on simule avec un gros tableau statique en mémoire
static mut DISK_STORAGE: [u8; 10 * 1024 * 1024] = [0; 10 * 1024 * 1024];

// Test complet de notre allocateur bump personnalisé
// Reproduit les tests de base + test de débordement volontaire
fn test_allocator() {
    println!("\n=== Test de l'allocateur personnalisé ===");
    
    // Test 1 : allocation d'un entier 32-bit
    let a = Box::new(42u32);
    println!("a = {}, address = {:p}, size = {} octets", a, a, mem::size_of_val(&*a));

    // Test 2 : allocation d'un tableau de 128 octets
    let b = Box::new([0u8; 128]);
    println!("b = [u8; 128], address = {:p}, size = {} octets", b.as_ptr(), mem::size_of_val(&*b));

    // Test 3 : allocation d'une string slice (la string elle-même est dans le binaire)
    let c = Box::new("hello rust");
    println!("c = {}, address = {:p}, size = {} octets", c, c.as_ptr(), mem::size_of_val(&*c));
    
    // Afficher l'état de notre allocateur
    println!("Mémoire utilisée : {} / {} octets", 
             allocator::ALLOCATOR.allocated_bytes(), 
             allocator::ALLOCATOR.heap_size());

    // Test 4 : tentative d'allocation qui doit échouer (plus de mémoire dispo)
    // Notre heap fait 64KB, on essaie d'allouer 64KB d'un coup -> doit échouer
    let layout = Layout::from_size_align(64 * 1024, 8).unwrap();
    unsafe {
        let ptr = alloc(layout);
        if ptr.is_null() {
            println!("✅ Allocation échouée comme prévu (plus assez de mémoire)");
        } else {
            println!("❌ ATTENTION : allocation réussie alors qu'elle ne devrait pas");
            // Note : si ça arrive, c'est qu'on a mal calculé la taille ou qu'il y a un bug
        }
    }
    
    println!("Mémoire après tentative : {} / {} octets", 
             allocator::ALLOCATOR.allocated_bytes(), 
             allocator::ALLOCATOR.heap_size());
}

// Terminal interactif pour tester notre système FAT32 en live
// Inspiré des shells Unix mais simplifié pour notre cas d'usage
// Commandes disponibles : ls, create, read, delete, info, space, check, demo, quit
fn terminal_interactif(fs: &mut Fat32FileSystem) {
    println!("\n🚀 === TERMINAL FAT32 INTERACTIF ===");
    println!("Tapez 'help' pour voir les commandes disponibles");
    
    // Boucle principale du terminal (REPL = Read-Eval-Print-Loop)
    loop {
        // Afficher le prompt (comme bash$ ou cmd>)
        print!("FAT32> ");
        io::stdout().flush().unwrap();  // Forcer l'affichage immédiat
        
        // Lire une ligne depuis stdin
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let input = input.trim();  // Supprimer les \n et espaces
                let parts: Vec<&str> = input.split_whitespace().collect();  // Parser les arguments
                
                if parts.is_empty() {
                    continue;  // Ligne vide, recommencer
                }
                
                // Dispatch vers la bonne commande (pattern matching ftw!)
                match parts[0].to_lowercase().as_str() {
                    "help" | "h" => {
                        println!("📖 Commandes disponibles:");
                        println!("  ls                    - Lister les fichiers");
                        println!("  create <nom> <contenu> - Creer un fichier");
                        println!("  read <nom>            - Lire un fichier");
                        println!("  delete <nom>          - Supprimer un fichier");
                        println!("  info                  - Informations systeme");
                        println!("  space                 - Espace disque");
                        println!("  check                 - Verifier le systeme");
                        println!("  demo                  - Lancer demo automatique");
                        println!("  quit | exit           - Quitter");
                    },
                    
                    // Commande ls : lister les fichiers (comme Unix ls)
                    "ls" | "list" => {
                        let files = fs.list_files();
                        if files.is_empty() {
                            println!("📁 Aucun fichier trouve");
                        } else {
                            println!("📁 Fichiers:");
                            for file in files {
                                println!("  📄 {} (cluster {}, {} octets)", file.name, file.cluster, file.size);
                            }
                        }
                    },
                    
                    // Commande create : créer un nouveau fichier ON FAIT PAS TOUCH ICI 🤡 
                    "create" => {
                        if parts.len() < 3 {
                            println!("❌ Usage: create <nom> <contenu>");
                            continue;
                        }
                        
                        let filename = parts[1];
                        let content = parts[2..].join(" ");  // Rejoindre tous les mots après le nom
                        
                        match fs.create_file_named(filename, content.as_bytes()) {
                            Ok(cluster) => println!("✅ Fichier '{}' cree dans le cluster {} ({} octets)", 
                                                   filename, cluster, content.len()),
                            Err(e) => println!("❌ Erreur: {}", e),
                        }
                    },
                    
                    // Commande read : afficher le contenu d'un fichier (comme Unix cat) ON FAIT PAS DE CAT NON PLUS ICI ! 🤡 
                    "read" => {
                        if parts.len() != 2 {
                            println!("❌ Usage: read <nom>");
                            continue;
                        }
                        
                        match fs.read_file_by_name(parts[1]) {
                            Ok(data) => {
                                let content = std::str::from_utf8(&data).unwrap_or("Donnees binaires");
                                println!("📖 Contenu de '{}':", parts[1]);
                                println!("\"{}\"", content);
                            },
                            Err(e) => println!("❌ Erreur: {}", e),
                        }
                    },
                    
                    // Commande delete : supprimer un fichier (comme Unix rm)
                    "delete" | "del" | "rm" => {
                        if parts.len() != 2 {
                            println!("❌ Usage: delete <nom>");
                            continue;
                        }
                        
                        match fs.delete_file_by_name(parts[1]) {
                            Ok(_) => println!("✅ Fichier '{}' supprime", parts[1]),
                            Err(e) => println!("❌ Erreur: {}", e),
                        }
                    },
                    
                    "info" => {
                        fs.info();
                    },
                    
                    "space" => {
                        match fs.get_free_space() {
                            Ok(free_space) => {
                                let total_space = fs.storage.len() as u32;
                                let used_space = total_space - free_space;
                                println!("💾 Espace disque:");
                                println!("  Total: {} octets ({} KB)", total_space, total_space / 1024);
                                println!("  Utilise: {} octets ({} KB)", used_space, used_space / 1024);
                                println!("  Libre: {} octets ({} KB)", free_space, free_space / 1024);
                            },
                            Err(e) => println!("❌ Erreur: {}", e),
                        }
                    },
                    
                    "check" => {
                        match fs.check_filesystem() {
                            Ok(_) => println!("✅ Systeme de fichiers OK"),
                            Err(e) => println!("❌ Erreur: {}", e),
                        }
                    },
                    
                    "demo" => {
                        test_fat32_demo(fs);
                    },
                    
                    "quit" | "exit" | "q" => {
                        println!("👋 Au revoir!");
                        break;
                    },
                    
                    _ => {
                        println!("❌ Commande inconnue: '{}'. Tapez 'help' pour l'aide.", parts[0]);
                    }
                }
            },
            Err(e) => {
                println!("❌ Erreur de lecture: {}", e);
                break;
            }
        }
    }
}

fn test_fat32_demo(fs: &mut Fat32FileSystem) {
    println!("\n🎬 === DEMONSTRATION AUTOMATIQUE ===");
    
    // Test de création de fichiers
    println!("\n--- Creation de fichiers de demonstration ---");
    
    let test_files: &[(&str, &[u8])] = &[
        ("HELLO.TXT", b"Hello, World! Ceci est un test FAT32."),
        ("TEST.DAT", b"Donnees de test pour le filesystem"),
        ("README.MD", b"# Projet FAT32\nAllocateur personnalise"),
    ];

    for (name, content) in test_files.iter() {
        // Vérifier si le fichier existe déjà
        if fs.find_file(name).is_some() {
            println!("ℹ️  Fichier {} existe deja", name);
            continue;
        }
        
        match fs.create_file_named(name, content) {
            Ok(cluster) => println!("✅ Fichier '{}' cree dans le cluster {} ({} octets)", 
                                   name, cluster, content.len()),
            Err(e) => println!("❌ Erreur creation {}: {}", name, e),
        }
    }
    
    // Test de lecture
    println!("\n--- Test de lecture ---");
    for (name, _) in test_files.iter() {
        match fs.read_file_by_name(name) {
            Ok(data) => {
                let content = std::str::from_utf8(&data).unwrap_or("Donnees binaires");
                println!("✅ Lecture {}: \"{}\"", name, content);
            },
            Err(e) => println!("❌ Erreur lecture {}: {}", name, e),
        }
    }
    
    // Vérification
    if let Err(e) = fs.check_filesystem() {
        println!("❌ Erreur verification: {}", e);
    }
    
    fs.summary();
    
    println!("✅ Demonstration terminee");
}

fn main() {
    println!("🚀 === SYSTEME FAT32 AVEC ALLOCATEUR PERSONNALISE ===");
    println!("Projet étudiant : Implémentation d'un filesystem sans libc malloc");
    println!("Architecture : Allocateur bump + FAT32 basique + Terminal interactif");
    
    // Étape 1 : Test de notre allocateur personnalisé
    test_allocator();
    
    // Étape 2 : Initialisation du système de fichiers FAT32
    println!("\n=== Initialisation du système FAT32 ===");
    let storage = unsafe { &mut DISK_STORAGE };  // Récupération de notre "disque"
    let mut fs = match Fat32FileSystem::new(storage) {
        Ok(fs) => {
            println!("✅ Systeme de fichiers FAT32 cree avec succes!");
            println!("   - Boot sector écrit (signature 0xAA55)");
            println!("   - Table FAT initialisée"); 
            println!("   - {} clusters disponibles", fs.total_clusters);
            fs
        },
        Err(e) => {
            println!("❌ Erreur lors de la creation du FS: {}", e);
            return;  // Abandon si on peut pas créer le FS
        }
    };
    
    // Étape 3 : Menu utilisateur (interface humaine)
    println!("\n📋 === MENU PRINCIPAL ===");
    println!("Choisissez votre mode d'interaction :");
    println!("[1] Demonstration automatique (fichiers pre-definis)");
    println!("[2] Terminal interactif (vous tapez les commandes)");
    print!("\nVotre choix (1 ou 2): ");
    io::stdout().flush().unwrap();
    
    // Lecture du choix utilisateur
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    
    // Dispatch selon le choix
    match input.trim() {
        "1" => {
            println!("\n🎬 Mode démonstration sélectionné");
            test_fat32_demo(&mut fs);
        },
        "2" => {
            println!("\n💻 Mode terminal interactif sélectionné"); 
            terminal_interactif(&mut fs);
        },
        _ => {
            println!("Choix invalide, lancement de la demonstration automatique...");
            test_fat32_demo(&mut fs);
        }
    }
    
    println!("\n🎉 === PROGRAMME TERMINE ===");
    println!("Merci d'avoir testé notre implémentation FAT32 !");
    // Note : pas besoin de free() grâce à notre allocateur bump 
    // (tout est libéré automatiquement à la fin du programme)
}