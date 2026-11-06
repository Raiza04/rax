//! File system structures and block management for the encrypted vault.
//!
//! This module defines the `Inode` and `InodeTable` structures, which act as
//! the "brain" of the virtual file system. It handles block allocation,
//! deallocation, and the serialization of the file allocation table.

use serde::{Deserialize, Serialize};

/// Represents the metadata and block mapping for a single file inside the vault.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Inode {
    /// The original name of the file (e.g., "document.txt").
    pub name: String,
    /// The exact size of the original plaintext file in bytes.
    pub size_bytes: u64,
    /// The creation time of the file represented as a Unix timestamp.
    pub created_at: u64,
    /// A list of logical block indices on the disk where the encrypted file data is stored.
    pub blocks: Vec<usize>,
}

/// The central directory of the vault, keeping track of all files and block allocations.
#[derive(Serialize, Deserialize, Debug)]
pub struct InodeTable {
    /// The list of all files currently stored in the vault.
    pub files: Vec<Inode>,
    /// The size of a single data block in bytes (defaults to 4096).
    pub block_size: usize,
    /// A bitmap tracking block availability. `true` means allocated, `false` means free.
    pub free_blocks: Vec<bool>,
}

impl InodeTable {
    /// Creates a new, empty `InodeTable` based on the total vault size.
    ///
    /// The first megabyte of the vault is always reserved for system metadata
    /// (magic bytes, salt, auth block, and the table itself).
    ///
    /// # Arguments
    /// * `size_mb` - The total size of the vault in megabytes.
    ///
    /// # Examples
    ///
    /// ```
    /// // Creates a table for a 2 MB vault (512 blocks total)
    /// let table = InodeTable::new(2);
    ///
    /// // The first 1 MB (256 blocks) is pre-allocated for system use
    /// assert_eq!(table.free_blocks[0], true);
    /// assert_eq!(table.free_blocks.len(), 512);
    /// ```
    pub fn new(size_mb: usize) -> Self {
        let block_size = 4096;

        // 1MB contains exactly 256 blocks based on a 4096-byte block size
        let total_blocks = size_mb * 256;

        let mut arr = vec![false; total_blocks];

        // Reserve the first 1MB (256 blocks) for system headers and the growing InodeTable
        arr[0..256].fill(true);

        Self {
            files: Vec::new(),
            block_size,
            free_blocks: arr,
        }
    }

    /// Finds and reserves the requested number of free blocks in the vault.
    ///
    /// Scans the `free_blocks` bitmap for available space and marks it as allocated
    /// if enough blocks are found.
    ///
    /// # Arguments
    /// * `blocks_needed` - The number of 4096-byte blocks required for the file.
    ///
    /// # Returns
    /// * `Some(Vec<usize>)` containing the allocated block indices if successful.
    /// * `None` if there is not enough free space in the vault.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut table = InodeTable::new(2);
    ///
    /// // Allocate space for a file needing 3 blocks
    /// let blocks = table.allocate(3).unwrap();
    /// assert_eq!(blocks.len(), 3);
    ///
    /// // The blocks are now marked as true (allocated)
    /// assert_eq!(table.free_blocks[blocks[0]], true);
    /// ```
    pub fn allocate(&mut self, blocks_needed: usize) -> Option<Vec<usize>> {
        let mut free_space = Vec::new();
        let mut counter = 0;

        for (i, &value) in self.free_blocks.iter().enumerate() {
            if counter == blocks_needed {
                break;
            }

            if !value {
                counter += 1;
                free_space.push(i);
            }
        }

        if counter == blocks_needed {
            for &i in &free_space {
                self.free_blocks[i] = true;
            }
        } else {
            return None;
        }

        Some(free_space)
    }

    /// Frees previously allocated blocks, making them available for new files.
    ///
    /// # Arguments
    /// * `blocks` - A slice of block indices to be marked as free (`false`).
    ///
    /// # Examples
    ///
    /// ```
    /// let mut table = InodeTable::new(2);
    /// let allocated = table.allocate(2).unwrap();
    ///
    /// // Free the blocks when deleting a file
    /// table.deallocate(&allocated);
    ///
    /// // The blocks are now available again
    /// assert_eq!(table.free_blocks[allocated[0]], false);
    /// ```
    pub fn deallocate(&mut self, blocks: &[usize]) {
        for &i in blocks {
            self.free_blocks[i] = false;
        }
    }

    /// Serializes the current state of the `InodeTable` into a raw byte vector.
    ///
    /// # Errors
    /// Returns an `std::io::Error` of kind `InvalidData` if the `bincode` serialization fails.
    pub fn to_bytes(&self) -> Result<Vec<u8>, std::io::Error> {
        bincode::serialize(self).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to serialize InodeTable: {}", e),
            )
        })
    }

    /// Deserializes a raw byte slice back into a functional `InodeTable`.
    ///
    /// # Arguments
    /// * `data` - The decrypted byte slice read from the vault's metadata section.
    ///
    /// # Errors
    /// Returns an `std::io::Error` of kind `InvalidData` if the `bincode` deserialization fails.
    pub fn from_bytes(data: &[u8]) -> Result<Self, std::io::Error> {
        bincode::deserialize(data).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to deserialize InodeTable: {}", e),
            )
        })
    }
}
