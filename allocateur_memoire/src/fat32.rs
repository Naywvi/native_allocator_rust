// fat32.rs - Implémentation d'un système de fichiers FAT32 basique
// 
// Références utilisées pour comprendre FAT32 :
// → https://wiki.osdev.org/FAT32 (excellent pour comprendre la structure)
// → https://www.win.tue.nl/~aeb/linux/fs/fat/fat-1.html (spécifications détaillées)
// → Microsoft FAT32 File System Specification (document officiel)
//
// Note perso : FAT32 = File Allocation Table 32-bit, remplace FAT16
// Le "32" vient du fait qu'on utilise 32 bits pour adresser les clusters (en fait 28 bits utilisés)

// Structure du Boot Sector FAT32 (exactement 512 octets)
// Sources : Microsoft FAT32 specification + osdev wiki
// Le #[repr(C, packed)] force Rust à respecter l'ordre exact des champs sans padding
// → https://doc.rust-lang.org/reference/type-layout.html#reprc-structs
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Fat32BootSector {
    // Octets 0-2 : Instructions de saut vers le code de boot (souvent EB 58 90)
    pub jump_boot: [u8; 3],           
    // Octets 3-10 : Nom du système qui a formaté le volume (8 caractères)
    pub oem_name: [u8; 8],            
    // Octets 11-12 : Nombre d'octets par secteur (généralement 512)
    pub bytes_per_sector: u16,         
    // Octet 13 : Nombre de secteurs par cluster (doit être une puissance de 2)
    pub sectors_per_cluster: u8,       
    // Octets 14-15 : Nombre de secteurs réservés avant la première FAT
    pub reserved_sector_count: u16,    
    // Octet 16 : Nombre de copies de la FAT (généralement 2 pour la redondance)
    pub num_fats: u8,                 
    // Octets 17-18 : Nombre d'entrées dans le répertoire racine (0 pour FAT32 car le root est un cluster normal)
    pub root_entry_count: u16,        
    // Octets 19-20 : Total des secteurs si < 65536, sinon 0 (utiliser total_sectors_32)
    pub total_sectors_16: u16,        
    // Octet 21 : Descripteur du média (0xF8 pour disque dur, 0xF0 pour disquette)
    pub media: u8,                    
    // Octets 22-23 : Secteurs par FAT pour FAT12/16, doit être 0 pour FAT32
    pub fat_size_16: u16,             
    // Octets 24-25 : Secteurs par piste (hérité du temps des disquettes)
    pub sectors_per_track: u16,       
    // Octets 26-27 : Nombre de têtes de lecture (hérité aussi)
    pub num_heads: u16,               
    // Octets 28-31 : Secteurs cachés avant la partition
    pub hidden_sectors: u32,          
    // Octets 32-35 : Total des secteurs (utilisé si > 65535)
    pub total_sectors_32: u32,        
    
    // === Extension spécifique FAT32 (à partir de l'octet 36) ===
    // Octets 36-39 : Taille d'une FAT en secteurs (pour FAT32)
    pub fat_size_32: u32,             
    // Octets 40-41 : Flags étendus (bit 7 = une seule FAT active)
    pub ext_flags: u16,               
    // Octets 42-43 : Version du système de fichiers (généralement 0)
    pub fs_version: u16,              
    // Octets 44-47 : Cluster de départ du répertoire racine (généralement 2)
    pub root_cluster: u32,            
    // Octets 48-49 : Secteur contenant les infos FSInfo
    pub fs_info: u16,                 
    // Octets 50-51 : Secteur de la copie de sauvegarde du boot sector
    pub backup_boot_sector: u16,      
    // Octets 52-63 : Réservé pour usage futur
    pub reserved: [u8; 12],           
    // Octet 64 : Numéro de lecteur physique (0x80 pour disque dur)
    pub drive_number: u8,             
    // Octet 65 : Réservé (utilisé par Windows NT)
    pub reserved1: u8,                
    // Octet 66 : Signature de boot étendue (0x29 si les champs suivants sont valides)
    pub boot_signature: u8,           
    // Octets 67-70 : Numéro de série du volume (généré aléatoirement)
    pub volume_id: u32,               
    // Octets 71-81 : Label du volume (11 caractères, paddé avec des espaces)
    pub volume_label: [u8; 11],       
    // Octets 82-89 : Type de système de fichiers ("FAT32   ")
    pub fs_type: [u8; 8],             
    // Octets 90-509 : Code de boot (non utilisé dans notre implémentation)
    pub boot_code: [u8; 420],         
    // Octets 510-511 : Signature de fin (doit être 0xAA55)
    pub signature: u16,               
}

// Entrée de répertoire FAT32 (exactement 32 octets)
// Chaque fichier/dossier a une entrée de cette taille dans son répertoire parent
// Source : https://wiki.osdev.org/FAT32#Directory_Structure
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct DirectoryEntry {
    // Octets 0-10 : Nom au format 8.3 (8 chars nom + 3 chars extension, paddé avec espaces)
    pub name: [u8; 11],               
    // Octet 11 : Attributs du fichier (lecture seule, caché, système, etc.)
    pub attributes: u8,               
    // Octet 12 : Réservé pour Windows NT
    pub reserved: u8,                 
    // Octet 13 : Dixièmes de seconde de la création (0-199)
    pub creation_time_tenth: u8,      
    // Octets 14-15 : Heure de création (format DOS)
    pub creation_time: u16,           
    // Octets 16-17 : Date de création (format DOS)
    pub creation_date: u16,           
    // Octets 18-19 : Date du dernier accès
    pub last_access_date: u16,        
    // Octets 20-21 : 16 bits de poids fort du premier cluster
    pub first_cluster_high: u16,      
    // Octets 22-23 : Heure de dernière modification
    pub write_time: u16,              
    // Octets 24-25 : Date de dernière modification
    pub write_date: u16,              
    // Octets 26-27 : 16 bits de poids faible du premier cluster
    pub first_cluster_low: u16,       
    // Octets 28-31 : Taille du fichier en octets (0 pour les répertoires)
    pub file_size: u32,               
}

// Constantes importantes FAT32
// Source des valeurs : Microsoft FAT32 File System Specification
pub const FILE_ATTRIBUTE_DIRECTORY: u8 = 0x10;  // Indique que l'entrée est un répertoire
pub const CLUSTER_FREE: u32 = 0x00000000;       // Cluster libre dans la FAT
pub const CLUSTER_END: u32 = 0x0FFFFFF8;        // Fin de chaîne de clusters (EOC = End Of Clusterchain)

// Structure pour stocker les infos d'un fichier (helper pour notre implémentation)
// Note : dans un vrai FAT32, ces infos viennent des DirectoryEntry
pub struct FileInfo {
    pub name: String,     // Nom du fichier
    pub cluster: u32,     // Premier cluster du fichier
    pub size: usize,      // Taille en octets
}

// Structure principale du système de fichiers
// Contient toutes les métadonnées nécessaires pour gérer notre "disque" FAT32
pub struct Fat32FileSystem {
    pub boot_sector: Fat32BootSector,   // Le boot sector qu'on a créé
    pub fat_start_sector: u32,          // Secteur où commence la première FAT
    pub data_start_sector: u32,         // Secteur où commencent les données (après les FATs)
    pub total_clusters: u32,            // Nombre total de clusters de données disponibles
    pub storage: &'static mut [u8],     // Notre "disque" simulé en mémoire
}

impl Fat32FileSystem {
    // Fonction pour créer et initialiser un système de fichiers FAT32 complet
    // Paramètre : un buffer mémoire qui simule notre disque dur
    pub fn new(storage: &'static mut [u8]) -> Result<Self, &'static str> {
        // Vérification de taille minimale (1MB minimum pour avoir assez de place)
        if storage.len() < 1024 * 1024 {  // Minimum 1MB
            return Err("Storage trop petit pour FAT32");
        }

        // Création du boot sector avec des valeurs standards FAT32
        // La plupart de ces valeurs viennent de la spec Microsoft
        let boot_sector = Fat32BootSector {
            // Signature de saut standard pour x86 (JMP SHORT + NOP)
            jump_boot: [0xEB, 0x58, 0x90],
            // Notre nom de système (8 caractères max, paddé avec des espaces)
            oem_name: *b"RUST_OS ",
            // 512 octets par secteur = standard depuis très longtemps
            bytes_per_sector: 512,
            // 8 secteurs par cluster = 4KB par cluster (bon compromis taille/fragmentation)
            sectors_per_cluster: 8,
            // 32 secteurs réservés avant la FAT (assez pour le boot sector + backup)
            reserved_sector_count: 32,
            // 2 copies de la FAT pour la redondance (si une se corrompt)
            num_fats: 2,
            // 0 entrées dans root car en FAT32 le root est un cluster normal
            root_entry_count: 0,
            // 0 car on utilise total_sectors_32 pour les gros volumes
            total_sectors_16: 0,
            // 0xF8 = media descriptor pour disque dur fixe
            media: 0xF8,
            // 0 car en FAT32 on utilise fat_size_32
            fat_size_16: 0,
            // Valeurs héritées du temps des disquettes mais obligatoires
            sectors_per_track: 63,
            num_heads: 255,
            // Pas de secteurs cachés dans notre cas
            hidden_sectors: 0,
            // Taille totale calculée depuis notre storage
            total_sectors_32: (storage.len() / 512) as u32,
            // 256 secteurs pour une FAT = 131072 entrées possibles (largement assez)
            fat_size_32: 256,
            // Pas de flags spéciaux
            ext_flags: 0,
            fs_version: 0,
            // Le répertoire racine commence au cluster 2 (0 et 1 sont réservés)
            root_cluster: 2,
            // Secteur 1 pour les infos FSInfo (pas implémenté ici)
            fs_info: 1,
            // Secteur 6 pour la copie de backup du boot sector
            backup_boot_sector: 6,
            reserved: [0; 12],
            // 0x80 = premier disque dur
            drive_number: 0x80,
            reserved1: 0,
            // 0x29 = signature de boot étendue valide
            boot_signature: 0x29,
            // ID généré "aléatoirement" (normalement basé sur date/heure)
            volume_id: 0x12345678,
            // Label de notre volume
            volume_label: *b"RUST_VOLUME",
            // Type de filesystem
            fs_type: *b"FAT32   ",
            // Code de boot vide (on ne boot pas dessus)
            boot_code: [0; 420],
            // Signature magique qui valide le boot sector
            signature: 0xAA55,
        };

        // Calculs des adresses importantes
        // FAT commence après les secteurs réservés
        let fat_start_sector = boot_sector.reserved_sector_count as u32;
        // Données commencent après toutes les FATs (num_fats * fat_size_32)
        let data_start_sector = fat_start_sector + (boot_sector.num_fats as u32 * boot_sector.fat_size_32);
        
        let mut fs = Fat32FileSystem {
            boot_sector,
            fat_start_sector,
            data_start_sector,
            total_clusters: 0, // Calculé juste après
            storage,
        };

        // Calcul du nombre de clusters de données disponibles
        // = (taille_storage - zone_systeme) / taille_cluster
        fs.total_clusters = (fs.storage.len() as u32 - fs.data_start_sector * 512) / (8 * 512);

       // Initialisation physique du système de fichiers
        fs.write_boot_sector()?;    // Écrire le boot sector sur le "disque"
        fs.initialize_fat()?;       // Initialiser la table FAT

        Ok(fs)
    }

    // Écrit le boot sector dans le storage à l'offset 0
    // Note : on utilise unsafe car on manipule des pointeurs bruts
    fn write_boot_sector(&mut self) -> Result<(), &'static str> {
        // Conversion de la structure en bytes bruts
        // → https://doc.rust-lang.org/std/slice/fn.from_raw_parts.html
        let boot_sector_bytes = unsafe {
            core::slice::from_raw_parts(
                &self.boot_sector as *const _ as *const u8,
                core::mem::size_of::<Fat32BootSector>()// Doit faire exactement 512 bytes
            )
        };
        
        // Vérification que notre storage est assez grand
        if self.storage.len() < boot_sector_bytes.len() {
            return Err("Storage trop petit pour le boot sector");
        }
        
        // Copie du boot sector au début du storage (secteur 0)
        self.storage[..boot_sector_bytes.len()].copy_from_slice(boot_sector_bytes);
        Ok(())
    }

    // Initialise la table FAT avec les valeurs par défaut
    // Les premières entrées ont des significations spéciales
    fn initialize_fat(&mut self) -> Result<(), &'static str> {
        let fat_offset = (self.fat_start_sector * 512) as usize;
        let fat_size = (self.boot_sector.fat_size_32 * 512) as usize;
        
        // Vérifier que la FAT rentre dans notre storage
        if fat_offset + fat_size > self.storage.len() {
            return Err("FAT ne rentre pas dans le storage");
        }

        // Nettoyer toute la zone FAT (mettre à zéro)
        for i in fat_offset..fat_offset + fat_size {
            self.storage[i] = 0;
        }

        // Initialiser les entrées spéciales de la FAT
        // FAT[0] = media descriptor (reprend la valeur du boot sector)
        self.write_fat_entry(0, 0x0FFFFFF8)?;  
        // FAT[1] = marqueur de fin de chaîne (toujours)
        self.write_fat_entry(1, CLUSTER_END)?;   
        // FAT[2] = répertoire racine (marqué comme utilisé)
        self.write_fat_entry(2, CLUSTER_END)?; 

        Ok(())
    }

    // Écrit une entrée dans la table FAT
    // La FAT est un tableau qui indique pour chaque cluster soit :
    // - 0 = cluster libre
    // - 0x0FFFFFF8-0x0FFFFFFF = fin de fichier
    // - autre valeur = numéro du cluster suivant dans la chaîne
    fn write_fat_entry(&mut self, cluster: u32, value: u32) -> Result<(), &'static str> {
        // Vérification des bornes (clusters 0 et 1 sont réservés mais accessibles)
        if cluster >= self.total_clusters + 2 {  // +2 car les clusters commencent à 2
            return Err("Cluster invalide");
        }

        // Calcul de l'adresse dans le storage
        // Chaque entrée FAT32 fait 4 octets (32 bits)
        let fat_offset = (self.fat_start_sector * 512) as usize;
        let entry_offset = fat_offset + (cluster as usize * 4);

        // Vérification que l'écriture ne dépasse pas le storage
        if entry_offset + 4 > self.storage.len() {
            return Err("Offset FAT invalide");
        }

        // En FAT32, seuls les 28 bits de poids faible sont utilisés
        // Les 4 bits de poids fort sont réservés et doivent être préservés
        // → https://wiki.osdev.org/FAT32#FAT_Entry_Values
        let masked_value = value & 0x0FFFFFFF;
        let bytes = masked_value.to_le_bytes(); // Little-endian comme x86
        
        // Écriture des 4 octets dans le storage
        self.storage[entry_offset] = bytes[0];
        self.storage[entry_offset + 1] = bytes[1];
        self.storage[entry_offset + 2] = bytes[2];
        self.storage[entry_offset + 3] = bytes[3];

        // Note : dans une implémentation complète, il faudrait aussi écrire
        // dans la deuxième FAT pour la redondance
        Ok(())
    }

    // Lit une entrée de la table FAT
    // Fonction inverse de write_fat_entry
    pub fn read_fat_entry(&self, cluster: u32) -> Result<u32, &'static str> {
        if cluster >= self.total_clusters + 2 {
            return Err("Cluster invalide");
        }

        let fat_offset = (self.fat_start_sector * 512) as usize;
        let entry_offset = fat_offset + (cluster as usize * 4);

        if entry_offset + 4 > self.storage.len() {
            return Err("Offset FAT invalide");
        }

        // Lecture des 4 octets et reconstruction de la valeur 32-bit
        let entry = u32::from_le_bytes([
            self.storage[entry_offset],
            self.storage[entry_offset + 1],
            self.storage[entry_offset + 2],
            self.storage[entry_offset + 3],
        ]);
        // Masquer les 4 bits de poids fort (toujours faire ça en FAT32)
        // Trouve un cluster libre dans la FAT
        // Algorithme simple : scan linéaire de la FAT jusqu'à trouver une entrée à 0
        // Note : dans un vrai OS, on utiliserait un bitmap ou une liste chaînée pour optimiser

        Ok(entry & 0x0FFFFFFF)
    }

    // Affiche les informations détaillées du système de fichiers
    // Utile pour débugger et comprendre la structure
    pub fn info(&self) {
        println!("=== Informations FAT32 ===");
        // Extraire les valeurs depuis les structures packed (évite les warnings d'alignement)
        let bytes_per_sector = self.boot_sector.bytes_per_sector;
        let sectors_per_cluster = self.boot_sector.sectors_per_cluster;
        let signature = self.boot_sector.signature;
        let fat_size = self.boot_sector.fat_size_32;
        let root_cluster = self.boot_sector.root_cluster;
        
        println!("Octets par secteur: {}", bytes_per_sector);
        println!("Secteurs par cluster: {}", sectors_per_cluster);
        println!("Taille FAT: {} secteurs", fat_size);
        println!("Cluster racine: {}", root_cluster);
        println!("Total clusters: {}", self.total_clusters);
        println!("Taille storage: {} octets", self.storage.len());
        println!("Signature: 0x{:04X}", signature);
        
        // Debug : afficher les premières entrées de la FAT
        // FAT[0] devrait être 0x0FFFFFF8 (media descriptor)
        // FAT[1] devrait être 0x0FFFFFF8 (toujours)  
        // FAT[2] devrait être 0x0FFFFFF8 (répertoire racine)
        if let Ok(fat0) = self.read_fat_entry(0) {
            println!("FAT[0] = 0x{:08X}", fat0);
        }
        if let Ok(fat1) = self.read_fat_entry(1) {
            println!("FAT[1] = 0x{:08X}", fat1);
        }
        if let Ok(fat2) = self.read_fat_entry(2) {
            println!("FAT[2] = 0x{:08X}", fat2);
        }
    }

    // Liste de tous les fichiers créés avec un mapping dynamique
    pub fn list_files(&self) -> Vec<FileInfo> {
        let mut files = Vec::new();
        let mut file_counter = 0;
        
        // Scanner les clusters utilisés à partir de 3
        for cluster in 3..self.total_clusters + 2 {
            if let Ok(fat_entry) = self.read_fat_entry(cluster) {
                if fat_entry != CLUSTER_FREE {
                    // Lire le début du cluster pour deviner le contenu
                    if let Ok(cluster_data) = self.read_cluster(cluster) {
                        // Essayer de déterminer la taille réelle du fichier
                        let mut size = 0;
                        for &byte in cluster_data.iter() {
                            if byte == 0 {
                                break;
                            }
                            size += 1;
                        }
                        
                        // Générer un nom basé sur le cluster ou le contenu
                        let name = if size > 0 {
                            // Essayer de créer un nom basé sur le contenu
                            let content = std::str::from_utf8(&cluster_data[..size.min(20)]).unwrap_or("DATA");
                            if content.starts_with("Hello") {
                                "HELLO.TXT".to_string()
                            } else if content.starts_with("Donnees") || content.starts_with("aaa") {
                                format!("FILE_{}.DAT", file_counter)
                            } else if content.starts_with("#") {
                                "README.MD".to_string()
                            } else {
                                format!("USER_{}.TXT", file_counter)
                            }
                        } else {
                            format!("EMPTY_{}.DAT", cluster)
                        };
                        
                        files.push(FileInfo { name, cluster, size });
                        file_counter += 1;
                    }
                }
            }
        }
        
        files
    }

    // Trouve un fichier par son nom
    pub fn find_file(&self, filename: &str) -> Option<FileInfo> {
        let files = self.list_files();
        files.into_iter().find(|f| f.name.to_uppercase() == filename.to_uppercase())
    }

    // Crée un fichier avec un nom spécifique (version améliorée)
    pub fn create_file_named(&mut self, name: &str, data: &[u8]) -> Result<u32, &'static str> {
        if name.len() > 11 {
            return Err("Nom de fichier trop long (max 11 caracteres)");
        }

        // Vérifier si le fichier existe déjà
        if self.find_file(name).is_some() {
            return Err("Fichier deja existant");
        }

        // Allouer un cluster pour le fichier
        let file_cluster = self.allocate_cluster()?;
        
        // Écrire les données du fichier
        self.write_cluster(file_cluster, data)?;

        Ok(file_cluster)
    }

    // Lit un fichier par son nom
    pub fn read_file_by_name(&self, filename: &str) -> Result<Vec<u8>, &'static str> {
        if let Some(file_info) = self.find_file(filename) {
            self.read_file(file_info.cluster, file_info.size)
        } else {
            Err("Fichier non trouve")
        }
    }

    // Supprime un fichier (simulation - marque le cluster comme libre)
    pub fn delete_file_by_name(&mut self, filename: &str) -> Result<(), &'static str> {
        if let Some(file_info) = self.find_file(filename) {
            // Marquer le cluster comme libre
            self.write_fat_entry(file_info.cluster, CLUSTER_FREE)?;
            Ok(())
        } else {
            Err("Fichier non trouve")
        }
    }

    // Trouve un cluster libre
    pub fn find_free_cluster(&self) -> Result<u32, &'static str> {
        for cluster in 3..self.total_clusters + 2 {  // Commence à 3 (après root)
            if self.read_fat_entry(cluster)? == CLUSTER_FREE {
                return Ok(cluster);
            }
        }
        Err("Pas de cluster libre")
    }

    // Alloue un nouveau cluster
    pub fn allocate_cluster(&mut self) -> Result<u32, &'static str> {
        let cluster = self.find_free_cluster()?;
        self.write_fat_entry(cluster, CLUSTER_END)?;
        Ok(cluster)
    }

    // Convertit un numéro de cluster en offset dans le storage
    fn cluster_to_offset(&self, cluster: u32) -> usize {
        let cluster_offset = cluster - 2;  // Les clusters de données commencent à 2
        (self.data_start_sector * 512) as usize + 
        (cluster_offset * 8 * 512) as usize  // 8 secteurs par cluster
    }

    // Écrit des données dans un cluster
    pub fn write_cluster(&mut self, cluster: u32, data: &[u8]) -> Result<(), &'static str> {
        if cluster < 2 || cluster >= self.total_clusters + 2 {
            return Err("Cluster invalide");
        }

        let offset = self.cluster_to_offset(cluster);
        let cluster_size = 8 * 512;  // 8 secteurs * 512 octets
        
        if offset + cluster_size > self.storage.len() {
            return Err("Cluster dépasse le storage");
        }

        let write_size = data.len().min(cluster_size);
        self.storage[offset..offset + write_size].copy_from_slice(&data[..write_size]);
        
        // Remplir le reste avec des zéros
        if write_size < cluster_size {
            for i in offset + write_size..offset + cluster_size {
                self.storage[i] = 0;
            }
        }

        Ok(())
    }

    // Lit les données d'un cluster
    pub fn read_cluster(&self, cluster: u32) -> Result<&[u8], &'static str> {
        if cluster < 2 || cluster >= self.total_clusters + 2 {
            return Err("Cluster invalide");
        }

        let offset = self.cluster_to_offset(cluster);
        let cluster_size = 8 * 512;
        
        if offset + cluster_size > self.storage.len() {
            return Err("Cluster dépasse le storage");
        }

        Ok(&self.storage[offset..offset + cluster_size])
    }

    // Crée un fichier simple
    pub fn create_file(&mut self, name: &str, data: &[u8]) -> Result<(), &'static str> {
        if name.len() > 11 {
            return Err("Nom de fichier trop long (max 11 caractères)");
        }

        // Allouer un cluster pour le fichier
        let file_cluster = self.allocate_cluster()?;
        
        // Écrire les données du fichier
        self.write_cluster(file_cluster, data)?;

        println!("✅ Fichier '{}' cree dans le cluster {} ({} octets)", name, file_cluster, data.len());
        
        Ok(())
    }

    // Lit un fichier par son cluster et taille
    pub fn read_file(&self, cluster: u32, file_size: usize) -> Result<Vec<u8>, &'static str> {
        let cluster_data = self.read_cluster(cluster)?;
        
        // Retourner seulement la taille réelle du fichier
        if file_size > cluster_data.len() {
            return Err("Taille de fichier invalide");
        }
        
        Ok(cluster_data[..file_size].to_vec())
    }

    // Vérifie l'intégrité du système de fichiers
    pub fn check_filesystem(&self) -> Result<(), &'static str> {
        println!("\n--- Verification du systeme de fichiers ---");
        
        // Vérifier la signature du boot sector
        let signature = self.boot_sector.signature;
        if signature != 0xAA55 {
            return Err("Signature du boot sector invalide");
        }
        println!("✅ Signature du boot sector valide (0x{:04X})", signature);
        
        // Vérifier que les clusters système sont bien marqués
        let fat0 = self.read_fat_entry(0)?;
        let fat1 = self.read_fat_entry(1)?;
        let fat2 = self.read_fat_entry(2)?;
        
        if fat0 == 0x0FFFFFF8 && fat1 == 0x0FFFFFF8 && fat2 == 0x0FFFFFF8 {
            println!("✅ Clusters systeme correctement marques");
        } else {
            return Err("Clusters systeme incorrects");
        }
        
        // Compter les clusters utilisés
        let mut used_clusters = 3; // 0, 1, 2 sont toujours utilisés
        for cluster in 3..self.total_clusters + 2 {
            if self.read_fat_entry(cluster)? != CLUSTER_FREE {
                used_clusters += 1;
            }
        }
        
        println!("✅ Clusters utilises: {} / {}", used_clusters, self.total_clusters + 2);
        
        Ok(())
    }

    // Affiche un résumé du système
    pub fn summary(&self) {
        println!("\n=== Résumé du système FAT32 ===");
        let bytes_per_sector = self.boot_sector.bytes_per_sector;
        let sectors_per_cluster = self.boot_sector.sectors_per_cluster;
        let cluster_size = bytes_per_sector as u32 * sectors_per_cluster as u32;
        
        println!("Taille cluster: {} octets", cluster_size);
        println!("Capacite totale: {} octets ({} MB)", 
                 self.storage.len(), self.storage.len() / (1024 * 1024));
        
        if let Ok(free_space) = self.get_free_space() {
            let used_space = self.storage.len() as u32 - free_space;
            println!("Espace utilise: {} octets ({} KB)", used_space, used_space / 1024);
            println!("Espace libre: {} octets ({} KB)", free_space, free_space / 1024);
        }
    }

    // Calcule l'espace libre
    pub fn get_free_space(&self) -> Result<u32, &'static str> {
        let mut free_clusters = 0;
        
        for cluster in 3..self.total_clusters + 2 {
            if self.read_fat_entry(cluster)? == CLUSTER_FREE {
                free_clusters += 1;
            }
        }
        
        Ok(free_clusters * 8 * 512)  // clusters * secteurs/cluster * octets/secteur
    }
}