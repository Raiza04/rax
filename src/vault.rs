use directories::ProjectDirs;
use rand::RngExt;
use std::fs::{self, File};
use std::io::{Error, ErrorKind, Read, Result, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

pub struct Vault {
    file: File,
    is_connected: bool,
    salt: [u8; 32],
    cipher: aes_gcm::Aes256Gcm,
    pub table: crate::fs::InodeTable,
}

impl Vault {
    pub fn create(path: &PathBuf, size_mb: usize, admin_pass: &str) -> Result<Vault> {
        let mut file = fs::File::create(path)?;

        // 1. Fill the vault with cryptographic noise
        let mut rng = rand::rng();
        let mut buffer = vec![0u8; 1024 * 1024];

        for _ in 0..size_mb {
            rng.fill(&mut buffer[..]);
            file.write_all(&buffer)?;
        }

        // 2. Rewind the file pointer to the beginning
        file.seek(SeekFrom::Start(0))?;

        // 3. Write header (Only Magic Bytes and Master Salt!)
        file.write_all(b"RAX1")?;

        let mut salt = [0u8; 32];
        rand::rng().fill(&mut salt);
        file.write_all(&salt)?;

        // 4. Initialize the crypto engine with the generated salt
        let cipher = crate::crypto::setup_cipher(admin_pass, &salt)?;

        // 5. Write the auth block (The bouncer)
        crate::crypto::write_encrypted_chunk(&mut file, &cipher, b"RAX_AUTH")?;

        // 6. Create and write the initial Inode Table (The brain)
        let table = crate::fs::InodeTable::new(size_mb);
        let table_bytes = table.to_bytes()?;
        crate::crypto::write_encrypted_chunk(&mut file, &cipher, &table_bytes)?;

        Ok(Vault {
            file,
            is_connected: true,
            salt,
            cipher,
            table,
        })
    }

    pub fn open(path: &PathBuf, pass_auth: &str) -> Result<Vault> {
        let mut f = fs::OpenOptions::new().read(true).write(true).open(path)?;

        // 1. Verify Magic Bytes
        let mut magic_bytes = [0u8; 4];
        f.read_exact(&mut magic_bytes)?;

        if &magic_bytes != b"RAX1" {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "vault.rax is corrupted (Invalid Magic Bytes)",
            ));
        }

        // 2. Read the master salt
        let mut salt_header = [0u8; 32];
        f.read_exact(&mut salt_header)?;

        // 3. Initialize the crypto engine
        let cipher = crate::crypto::setup_cipher(pass_auth, &salt_header)?;

        // 4. Read and decrypt the auth block
        let auth_decrypted = crate::crypto::read_encrypted_chunk(&mut f, &cipher)?;

        if auth_decrypted != b"RAX_AUTH" {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Access Denied: Auth Block corrupted!",
            ));
        }

        // 5. Read and deserialize the Inode Table
        let table_bytes = crate::crypto::read_encrypted_chunk(&mut f, &cipher)?;
        let table = crate::fs::InodeTable::from_bytes(&table_bytes)?;

        Ok(Vault {
            file: f,
            is_connected: true,
            salt: salt_header,
            cipher,
            table,
        })
    }

    /// Jumps to the physical offset of the given block index and writes EXACTLY 4096 bytes.
    pub fn write_block(&mut self, block_index: usize, data: &[u8; 4096]) -> Result<()> {
        let offset = block_index * 4096;
        self.file.seek(SeekFrom::Start(offset as u64))?;
        self.file.write_all(data)?;

        Ok(())
    }

    /// Jumps to the physical offset and reads EXACTLY 4096 bytes into a buffer.
    pub fn read_block(&mut self, block_index: usize) -> Result<[u8; 4096]> {
        let offset = block_index * 4096;
        self.file.seek(SeekFrom::Start(offset as u64))?;

        let mut buffer = [0u8; 4096];
        self.file.read_exact(&mut buffer)?;

        Ok(buffer)
    }

    /// Serializes the current InodeTable, encrypts it, and writes it back to disk at offset 80.
    pub fn sync(&mut self) -> Result<()> {
        let data = self.table.to_bytes()?;

        //Table starts at 80 after magic bytes, salt and auth bytes
        self.file.seek(SeekFrom::Start(80))?;

        crate::crypto::write_encrypted_chunk(&mut self.file, &self.cipher, data.as_ref())?;

        self.file.flush()?;

        Ok(())
    }

    /// Imports a plaintext file from the OS, encrypts it into blocks, and saves it in the vault.
    pub fn put_file(&mut self, source_path: &str, remove_sorce: &bool) -> Result<()> {
        let mut source_file = fs::File::open(source_path)?;

        let metadata = source_file.metadata()?;
        let file_size = metadata.len() as usize;

        // Math trick: Round up division (e.g., 5000 bytes / 4068 = 2 blocks)
        let blocks_needed = file_size.div_ceil(4068);

        let allocated_blocks = self
            .table
            .allocate(blocks_needed)
            .ok_or_else(|| Error::other("Not enough space in vault!!"))?;

        let mut buffer = [0u8; 4068];
        for block_index in allocated_blocks.iter() {
            let bytes_read = source_file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            } else if bytes_read < 4068 {
                buffer[bytes_read..].fill(0);
            }

            let payload = crate::crypto::encrypt_block_payload(&self.cipher, &buffer)?;
            self.write_block(*block_index, &payload)?;
        }

        let path = std::path::Path::new(source_path);
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown_file")
            .to_string();

        let create_time = metadata
            .created()?
            .duration_since(UNIX_EPOCH)
            .expect("The SystemTime is before 1970!");

        let created_at = create_time.as_secs();

        let new_inode = crate::fs::Inode {
            name: filename,
            size_bytes: file_size as u64,
            created_at,
            blocks: allocated_blocks,
        };

        self.table.files.push(new_inode);

        if *remove_sorce {
            std::fs::remove_file(source_path)?;
        }

        println!("File Successfully encrypted and stored");

        Ok(())
    }
}

pub fn get_vault_path() -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("", "", "rax").ok_or_else(|| {
        Error::new(
            ErrorKind::NotFound,
            "Could not find the user's home directory",
        )
    })?;

    let data_dir = proj_dirs.data_dir();
    // Create the directory and bubble up any errors with ?
    fs::create_dir_all(data_dir)?;

    Ok(data_dir.join("vault.rax"))
}
