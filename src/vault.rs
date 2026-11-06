//! Core implementation of the encrypted Virtual File System (VFS).
//!
//! This module handles the physical file I/O, cryptographic boundaries,
//! block allocations, and persistent state management of the vault.

use directories::ProjectDirs;
use rand::RngExt;
use std::fs::{self, File};
use std::io::{Error, ErrorKind, Read, Result, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

/// Represents an open, authenticated session with the encrypted vault.
///
/// Holds the underlying file handle, cryptographic keys, and the
/// in-memory representation of the file system table (`InodeTable`).
pub struct Vault {
    /// The physical file handle to the vault on the host OS.
    file: File,
    /// The AES-256-GCM cipher instance used for block encryption/decryption.
    cipher: aes_gcm::Aes256Gcm,
    /// The central directory holding metadata and block allocations.
    pub table: crate::fs::InodeTable,
}

impl Vault {
    /// Creates a new, fully encrypted vault from scratch.
    ///
    /// This process involves generating cryptographic noise to fill the file,
    /// setting up the magic bytes (`RAX1`), generating a master salt, and
    /// writing the initial authentication block and `InodeTable`.
    ///
    /// # Arguments
    /// * `path` - The target location for the new vault file.
    /// * `size_mb` - The total size of the vault in megabytes.
    /// * `admin_pass` - The plaintext password used to derive the encryption key.
    ///
    /// # Errors
    /// Returns an `std::io::Error` if the file cannot be created, if I/O operations fail,
    /// or if cryptographic initialization fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::PathBuf;
    ///
    /// let vault_path = PathBuf::from("/home/user/.local/share/rax/vault.rax");
    /// // Creates a 50 MB encrypted vault
    /// let vault = Vault::create(&vault_path, 50, "super_secret_password").unwrap();
    /// ```
    pub fn create(path: &PathBuf, size_mb: usize, admin_pass: &str) -> Result<Vault> {
        let mut file = fs::File::create(path)?;

        // 1. Fill the entire vault with cryptographic noise to hide actual data boundaries
        let mut rng = rand::rng();
        let mut buffer = vec![0u8; 1024 * 1024];

        for _ in 0..size_mb {
            rng.fill(&mut buffer[..]);
            file.write_all(&buffer)?;
        }

        // 2. Rewind the file pointer to the beginning to write the header
        file.seek(SeekFrom::Start(0))?;

        // 3. Write header: Magic Bytes identifier and Master Salt
        file.write_all(b"RAX1")?;

        let mut salt = [0u8; 32];
        rand::rng().fill(&mut salt);
        file.write_all(&salt)?;

        // 4. Initialize the crypto engine with the generated salt
        let cipher = crate::crypto::setup_cipher(admin_pass, &salt)?;

        // 5. Write the auth block (The "bouncer" verifying future access)
        crate::crypto::write_encrypted_chunk(&mut file, &cipher, b"RAX_AUTH")?;

        // 6. Create and write the initial InodeTable (The "brain" of the VFS)
        let table = crate::fs::InodeTable::new(size_mb);
        let table_bytes = table.to_bytes()?;
        crate::crypto::write_encrypted_chunk(&mut file, &cipher, &table_bytes)?;

        Ok(Vault {
            file,
            cipher,
            table,
        })
    }

    /// Opens and authenticates an existing vault.
    ///
    /// Verifies the magic bytes, reads the salt, derives the key, and attempts
    /// to decrypt the authentication block. If successful, it loads the `InodeTable` into memory.
    ///
    /// # Arguments
    /// * `path` - The path to the existing vault file.
    /// * `pass_auth` - The plaintext password to attempt authentication.
    ///
    /// # Errors
    /// Returns an error if the file doesn't exist, magic bytes mismatch,
    /// authentication fails (wrong password), or the table cannot be parsed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::PathBuf;
    ///
    /// let vault_path = PathBuf::from("/home/user/.local/share/rax/vault.rax");
    /// // Opens the vault and loads the InodeTable into RAM
    /// let mut vault = Vault::open(&vault_path, "super_secret_password").expect("Access Denied");
    /// ```
    pub fn open(path: &PathBuf, pass_auth: &str) -> Result<Vault> {
        let mut f = fs::OpenOptions::new().read(true).write(true).open(path)?;

        // 1. Verify Magic Bytes to ensure this is a valid rax vault
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

        // 3. Initialize the crypto engine using the stored salt
        let cipher = crate::crypto::setup_cipher(pass_auth, &salt_header)?;

        // 4. Read and decrypt the auth block to verify the password
        let auth_decrypted = crate::crypto::read_encrypted_chunk(&mut f, &cipher)?;

        if auth_decrypted != b"RAX_AUTH" {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Access Denied: Auth Block corrupted or wrong password!",
            ));
        }

        // 5. Read and deserialize the InodeTable into memory
        let table_bytes = crate::crypto::read_encrypted_chunk(&mut f, &cipher)?;
        let table = crate::fs::InodeTable::from_bytes(&table_bytes)?;

        Ok(Vault {
            file: f,
            cipher,
            table,
        })
    }

    /// Writes exactly 4096 bytes to the physical offset corresponding to the given block index.
    ///
    /// # Arguments
    /// * `block_index` - The logical block index determined by the `InodeTable`.
    /// * `data` - The 4096-byte encrypted payload and metadata to write.
    pub fn write_block(&mut self, block_index: usize, data: &[u8; 4096]) -> Result<()> {
        let offset = block_index * 4096;
        self.file.seek(SeekFrom::Start(offset as u64))?;
        self.file.write_all(data)?;

        Ok(())
    }

    /// Reads exactly 4096 bytes from the physical offset corresponding to the given block index.
    ///
    /// # Arguments
    /// * `block_index` - The logical block index to read from.
    pub fn read_block(&mut self, block_index: usize) -> Result<[u8; 4096]> {
        let offset = block_index * 4096;
        self.file.seek(SeekFrom::Start(offset as u64))?;

        let mut buffer = [0u8; 4096];
        self.file.read_exact(&mut buffer)?;

        Ok(buffer)
    }

    /// Serializes the current `InodeTable`, encrypts it, and writes it back to disk.
    ///
    /// This method ensures that file operations (like allocating blocks or adding files)
    /// are persistently saved to the physical vault file.
    ///
    /// # Errors
    /// Returns an error if serialization, encryption, or disk I/O fails.
    pub fn sync(&mut self) -> Result<()> {
        let data = self.table.to_bytes()?;

        // The InodeTable always starts at offset 80 (after magic bytes, salt, and auth chunk)
        self.file.seek(SeekFrom::Start(80))?;
        crate::crypto::write_encrypted_chunk(&mut self.file, &self.cipher, data.as_ref())?;
        self.file.flush()?;

        Ok(())
    }

    /// Imports a plaintext file from the host OS, encrypts it block-by-block, and stores it in the vault.
    ///
    /// Allocates necessary blocks, pads the last block if needed, encrypts the payload,
    /// and registers a new `Inode` in the table. Optionally removes the original file.
    ///
    /// # Arguments
    /// * `source_path` - Path to the plaintext file on the host OS.
    /// * `remove_sorce` - If true, securely removes the original file after successful import.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Assuming `vault` is a previously opened Vault instance
    /// let remove_after_import = true;
    /// vault.put_file("/path/to/my_secret_document.pdf", &remove_after_import).unwrap();
    /// // The file is now encrypted in the vault and the original is deleted
    /// vault.sync().unwrap();
    /// ```
    pub fn put_file(&mut self, source_path: &str, remove_sorce: &bool) -> Result<()> {
        let mut source_file = fs::File::open(source_path)?;

        let metadata = source_file.metadata()?;
        let file_size = metadata.len() as usize;

        // Calculate blocks using round-up division (e.g., 5000 bytes / 4068 = 2 blocks)
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
                // Pad the remaining buffer with zeros to avoid leaking old memory data
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

        Ok(())
    }

    /// Extracts a file from the vault and saves it decrypted to the host OS.
    ///
    /// # Arguments
    /// * `filename` - The name of the file inside the vault.
    /// * `dest_path` - The destination path on the host OS to save the decrypted file.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Assuming `vault` is a previously opened Vault instance
    /// vault.get_file("my_secret_document.pdf", "/tmp/extracted_document.pdf").unwrap();
    /// ```
    pub fn get_file(&mut self, filename: &str, dest_path: &str) -> std::io::Result<()> {
        let inode = self
            .table
            .files
            .iter()
            .find(|f| f.name == filename)
            .cloned()
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "File not found in vault")
            })?;

        let mut out_file = std::fs::File::create(dest_path)?;
        let mut bytes_left = inode.size_bytes as usize;

        for &block_index in &inode.blocks {
            let encrypted_block = self.read_block(block_index)?;
            let payload = crate::crypto::decrypt_block_payload(&self.cipher, &encrypted_block)?;

            // Prevent trailing zero-padding from being written to the extracted file
            let write_len = std::cmp::min(bytes_left, 4068);
            out_file.write_all(&payload[..write_len])?;

            bytes_left -= write_len;
        }

        Ok(())
    }

    /// Extracts a file from the vault directly into RAM without touching the host OS disk.
    ///
    /// # Arguments
    /// * `filename` - The name of the file inside the vault.
    ///
    /// # Errors
    /// Returns an error if the file is not found or decryption fails.
    pub fn read_file_to_memory(&mut self, filename: &str) -> Result<Vec<u8>> {
        let inode = self
            .table
            .files
            .iter()
            .find(|f| f.name == filename)
            .cloned()
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "File not found in vault"))?;

        let mut bytes_left = inode.size_bytes as usize;
        // Pre-allocate the exact amount of RAM needed
        let mut memory_buffer = Vec::with_capacity(bytes_left);

        for &block_index in &inode.blocks {
            let encrypted_block = self.read_block(block_index)?;
            let payload = crate::crypto::decrypt_block_payload(&self.cipher, &encrypted_block)?;

            let write_len = std::cmp::min(bytes_left, 4068);
            // Append the extracted slice directly to our RAM buffer
            memory_buffer.extend_from_slice(&payload[..write_len]);

            bytes_left -= write_len;
        }

        Ok(memory_buffer)
    }
}

/// Resolves the absolute path to the main vault file within the user's data directory.
///
/// Follows OS-specific conventions (e.g., `~/.local/share/rax/vault.rax` on Linux).
/// Ensures the parent directories exist before returning the path.
///
/// # Errors
/// Returns an error if the user's home directory cannot be determined or
/// if the directories cannot be created.
///
/// # Examples
///
/// ```no_run
/// let path = get_vault_path().unwrap();
/// println!("Vault will be stored at: {:?}", path);
/// ```
pub fn get_vault_path() -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("", "", "rax").ok_or_else(|| {
        Error::new(
            ErrorKind::NotFound,
            "Could not find the user's home directory",
        )
    })?;

    let data_dir = proj_dirs.data_dir();
    fs::create_dir_all(data_dir)?;

    Ok(data_dir.join("vault.rax"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use tempfile::tempdir;

    /// Tests the lifecycle of creating a vault and subsequently opening it.
    #[test]
    fn test_vault_lifecycle() {
        let dir = tempdir().unwrap();
        let vault_path = dir.path().join("vault.rax");

        let vault = Vault::create(&vault_path, 2, "admin123").unwrap();
        assert!(vault.table.free_blocks[0]);

        let opened_vault = Vault::open(&vault_path, "admin123").unwrap();
        assert_eq!(opened_vault.table.block_size, 4096);
    }

    /// Tests if the vaults opens with the wrong password
    #[test]
    fn test_open_wrong_password() {
        let dir = tempdir().unwrap();
        let vault_path = dir.path().join("vault.rax");

        Vault::create(&vault_path, 2, "admin123").unwrap();

        let result = Vault::open(&vault_path, "wrong_password");
        assert!(result.is_err());
    }

    /// Tests if the program handles corrupted files correctly
    #[test]
    fn test_corrupted_magic_bytes() {
        let dir = tempdir().unwrap();
        let vault_path = dir.path().join("vault.rax");

        Vault::create(&vault_path, 2, "admin123").unwrap();

        // Maliciously overwrite the Magic Bytes without triggering the crypto engine
        let mut file = OpenOptions::new().write(true).open(&vault_path).unwrap();
        file.write_all(b"HACK").unwrap();
        file.sync_all().unwrap();

        let result = Vault::open(&vault_path, "admin123");
        assert!(result.is_err());
    }

    /// End-to-end integration test: importing a file, checking RAM, and extracting it.
    #[test]
    fn test_file_import_and_extraction() {
        let dir = tempdir().unwrap();
        let vault_path = dir.path().join("vault.rax");

        Vault::create(&vault_path, 2, "admin123").unwrap();
        let mut vault = Vault::open(&vault_path, "admin123").unwrap();

        let source_path = dir.path().join("secret.txt");
        fs::write(&source_path, b"Top Secret File Content").unwrap();

        let remove = false;
        vault
            .put_file(source_path.to_str().unwrap(), &remove)
            .unwrap();

        assert_eq!(vault.table.files.len(), 1);
        assert_eq!(vault.table.files[0].name, "secret.txt");

        let ram_bytes = vault.read_file_to_memory("secret.txt").unwrap();
        assert_eq!(ram_bytes[0..23], b"Top Secret File Content"[..]);

        let dest_path = dir.path().join("extracted.txt");
        vault
            .get_file("secret.txt", dest_path.to_str().unwrap())
            .unwrap();

        let extracted = fs::read(&dest_path).unwrap();
        assert_eq!(extracted[0..23], b"Top Secret File Content"[..]);
    }
}
