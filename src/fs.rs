use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Inode {
    pub name: String,
    pub size_bytes: u64,
    pub created_at: u64,    // e.g. Unix-Timestamp
    pub blocks: Vec<usize>, // e.g. [5, 6, 7, 8]
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InodeTable {
    pub files: Vec<Inode>,
    pub block_size: usize,      // 4096 Bytes by default
    pub free_blocks: Vec<bool>, // true = allocated, false = free
}

impl InodeTable {
    pub fn new(size_mb: usize) -> Self {
        let block_size = 4096;

        // 1MB has exactly 256 block based on a block_size of 4096
        let total_blocks = size_mb * 256;

        let mut arr = vec![false; total_blocks];

        // We set the first MB true to reserve it for the "system"
        // since the inodetable can grow in size
        arr[0..256].fill(true);

        Self {
            files: Vec::new(),
            block_size,
            free_blocks: arr,
        }
    }

    //returns an vector of free blocks
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

    // sets the values in free_blocks to false to show
    // that the are no longer allocated
    pub fn deallocate(&mut self, blocks: &[usize]) {
        for i in blocks {
            self.free_blocks[*i] = false;
        }
    }

    // translates the table in raw bytes (Serialize)
    pub fn to_bytes(&self) -> Result<Vec<u8>, std::io::Error> {
        // bincode has its own error so we want to translate it
        // into std::io error to work with
        bincode::serialize(self).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to serialize InodeTable: {}", e),
            )
        })
    }

    // reads the bytes and builds the table together (Deserialize)
    pub fn from_bytes(data: &[u8]) -> Result<Self, std::io::Error> {
        bincode::deserialize(data).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to deserialize InodeTable: {}", e),
            )
        })
    }
}
