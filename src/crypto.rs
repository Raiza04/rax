//! Cryptographic engine for the `rax` Virtual File System.
//!
//! This module handles all encryption, decryption, and key derivation processes.
//! It uses **Argon2** to securely hash user passwords and derive strong master keys,
//! and **AES-256-GCM** (Galois/Counter Mode) to ensure both data confidentiality
//! and authenticity (tamper-proofing).
//!
//! It exposes two distinct data formats:
//! 1. **Fixed Blocks (`encrypt_block_payload`):** For raw file data mapped to the 4096-byte grid.
//! 2. **Dynamic Chunks (`write_encrypted_chunk`):** Variable-sized packets for system metadata (like the `InodeTable`).

use aes_gcm::{
    Aes256Gcm, Key as AesKey, Nonce as AesNonce,
    aead::{Aead, KeyInit},
};
use argon2::Argon2;

use rand::RngExt;
use std::fs::File;
use std::io::{Error, ErrorKind, Read, Result, Write};

/// Derives a 256-bit AES master key from a plaintext password and initializes the cipher.
///
/// This function uses the memory-hard `Argon2` algorithm to protect against
/// brute-force and dictionary attacks.
///
/// # Arguments
/// * `password` - The plaintext password provided by the user.
/// * `salt` - A 32-byte cryptographic salt read from the vault header.
///
/// # Errors
/// Returns an `std::io::Error` of kind `InvalidInput` if the Argon2 hashing process fails.
///
/// # Examples
///
/// ```no_run
/// // Assuming a 32-byte salt read from the vault
/// let salt = [0u8; 32];
/// let cipher = setup_cipher("my_admin_password", &salt).unwrap();
/// ```
pub fn setup_cipher(password: &str, salt: &[u8; 32]) -> Result<Aes256Gcm> {
    let mut key_bytes = [0u8; 32];
    let argon2 = Argon2::default();

    if argon2
        .hash_password_into(password.as_bytes(), salt, &mut key_bytes)
        .is_err()
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Failed to hash password",
        ));
    }

    let key = AesKey::<Aes256Gcm>::from(key_bytes);
    Ok(Aes256Gcm::new(&key))
}

/// Encrypts a fixed 4068-byte file payload into a full 4096-byte physical storage block.
///
/// The resulting 4096-byte block structure is:
/// * `[0..12]` - Randomly generated Nonce.
/// * `[12..4096]` - Ciphertext + 16-byte MAC Tag attached by AES-GCM.
///
/// # Security
/// A unique 12-byte Nonce is generated via a secure RNG for every single block
/// to ensure that identical payloads yield completely different ciphertexts.
///
/// # Arguments
/// * `cipher` - The initialized AES-256-GCM engine.
/// * `payload` - Exactly 4068 bytes of plaintext file data (padded with zeros if necessary).
///
/// # Errors
/// Returns an error if the underlying AES encryption fails.
pub fn encrypt_block_payload(cipher: &Aes256Gcm, payload: &[u8; 4068]) -> Result<[u8; 4096]> {
    let mut nonce = [0u8; 12];
    rand::rng().fill(&mut nonce);

    let aes_nonce = AesNonce::from(nonce);

    let ciphertext = cipher
        .encrypt(&aes_nonce, payload.as_ref())
        .map_err(|_| Error::other("Encryption failed"))?;

    let mut result = [0u8; 4096];
    result[0..12].copy_from_slice(&nonce);
    result[12..4096].copy_from_slice(&ciphertext);

    Ok(result)
}

/// Decrypts a 4096-byte physical storage block and extracts the original 4068-byte payload.
///
/// Validates the AES-GCM MAC Tag during decryption to detect any tampering or corruption.
///
/// # Arguments
/// * `cipher` - The initialized AES-256-GCM engine.
/// * `block` - The raw 4096-byte block read from the disk.
///
/// # Errors
/// Returns an `std::io::Error` of kind `PermissionDenied` if decryption fails
/// (e.g., wrong password, tampered data, or corrupted block).
pub fn decrypt_block_payload(cipher: &Aes256Gcm, block: &[u8; 4096]) -> Result<[u8; 4068]> {
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&block[0..12]);
    let aes_nonce = AesNonce::from(nonce);

    let mut data = [0u8; 4084];
    data.copy_from_slice(&block[12..4096]);

    let payloadtext = cipher.decrypt(&aes_nonce, data.as_ref()).map_err(|_| {
        Error::new(
            ErrorKind::PermissionDenied,
            "Access Denied: Wrong Password or corrupted data!",
        )
    })?;

    // 4084 total ciphertext bytes - 16 bytes (MAC Tag) = 4068 bytes of actual payload
    let mut result = [0u8; 4068];
    result.copy_from_slice(&payloadtext);

    Ok(result)
}

/// Encrypts variable-length metadata and writes it as a self-describing packet to the file.
///
/// Use this method for system headers (like the Auth Block or `InodeTable`),
/// not for mass file data. The packet layout on disk is:
/// `[Size (8 bytes, Little Endian)] + [Nonce (12 bytes)] + [Ciphertext + MAC Tag (Variable)]`
///
/// # Arguments
/// * `file` - A mutable reference to the open vault file.
/// * `cipher` - The initialized AES-256-GCM engine.
/// * `data` - The plaintext metadata byte slice to encrypt.
///
/// # Errors
/// Returns an error if encryption or disk write operations fail.
pub fn write_encrypted_chunk(file: &mut File, cipher: &Aes256Gcm, data: &[u8]) -> Result<()> {
    let mut nonce_bytes = [0u8; 12];
    rand::rng().fill(&mut nonce_bytes);
    let aes_nonce = AesNonce::from(nonce_bytes);

    let ciphertext = cipher
        .encrypt(&aes_nonce, data)
        .map_err(|_| Error::other("Encryption failed"))?;

    let size = ciphertext.len() as u64;

    file.write_all(&size.to_le_bytes())?;
    file.write_all(&nonce_bytes)?;
    file.write_all(&ciphertext)?;

    Ok(())
}

/// Reads a self-describing encrypted metadata packet from the file and decrypts it.
///
/// Parses the 8-byte size header to determine how many bytes to read,
/// extracts the nonce, and authenticates/decrypts the payload.
///
/// # Arguments
/// * `file` - A mutable reference to the open vault file positioned at the start of the chunk.
/// * `cipher` - The initialized AES-256-GCM engine.
///
/// # Errors
/// Returns an error if disk reads fail or if decryption/authentication fails.
pub fn read_encrypted_chunk(file: &mut File, cipher: &Aes256Gcm) -> Result<Vec<u8>> {
    // 1. Read the size of the incoming ciphertext (8 Bytes, Little Endian)
    let mut size_bytes = [0u8; 8];
    file.read_exact(&mut size_bytes)?;
    let size = u64::from_le_bytes(size_bytes) as usize;

    // 2. Read the specific nonce used for this chunk (12 Bytes)
    let mut nonce_bytes = [0u8; 12];
    file.read_exact(&mut nonce_bytes)?;
    let aes_nonce = AesNonce::from(nonce_bytes);

    // 3. Read the actual ciphertext based on the previously extracted size
    let mut ciphertext = vec![0u8; size];
    file.read_exact(&mut ciphertext)?;

    // 4. Authenticate and decrypt the data
    let decrypted = cipher
        .decrypt(&aes_nonce, ciphertext.as_ref())
        .map_err(|_| {
            Error::new(
                ErrorKind::PermissionDenied,
                "Access Denied: Wrong Password or corrupted data!",
            )
        })?;

    Ok(decrypted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Seek, SeekFrom};
    use tempfile::tempfile;

    /// Validates that a payload encrypts into a 4096-byte block and decrypts flawlessly.
    #[test]
    fn test_block_encryption_roundtrip() {
        let salt = [0u8; 32];
        let cipher = setup_cipher("test_password", &salt).unwrap();

        let mut payload = [0u8; 4068];
        payload[0..14].copy_from_slice(b"Secret Payload");

        let encrypted_block = encrypt_block_payload(&cipher, &payload).unwrap();
        assert_eq!(encrypted_block.len(), 4096);

        let decrypted_payload = decrypt_block_payload(&cipher, &encrypted_block).unwrap();
        assert_eq!(payload, decrypted_payload);
    }

    /// Tests if wrong passwords are handled correctly
    #[test]
    fn test_decryption_wrong_password() {
        let salt = [0u8; 32];
        let cipher_encrypt = setup_cipher("correct_password", &salt).unwrap();
        let cipher_decrypt = setup_cipher("hacker_password", &salt).unwrap();

        let payload = [42u8; 4068];
        let encrypted_block = encrypt_block_payload(&cipher_encrypt, &payload).unwrap();

        // Attempt to decrypt with the wrong cipher engine
        let result = decrypt_block_payload(&cipher_decrypt, &encrypted_block);
        assert!(result.is_err());
    }

    /// Validates variable-length chunk encryption using temporary files.
    #[test]
    fn test_chunk_encryption_roundtrip() {
        let salt = [0u8; 32];
        let cipher = setup_cipher("test_password", &salt).unwrap();
        let data = b"Vault Header Metadata";

        let mut file = tempfile().expect("Failed to create temp file");

        write_encrypted_chunk(&mut file, &cipher, data).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();

        let decrypted = read_encrypted_chunk(&mut file, &cipher).unwrap();
        assert_eq!(data.as_ref(), decrypted);
    }

    /// Ensures that modifying a ciphertext block triggers an Access Denied error (MAC mismatch).
    #[test]
    fn test_tamper_proofing() {
        let salt = [0u8; 32];
        let cipher = setup_cipher("test_password", &salt).unwrap();
        let payload = [42u8; 4068];

        let mut encrypted_block = encrypt_block_payload(&cipher, &payload).unwrap();

        // Simulate bit-rot or malicious tampering
        encrypted_block[100] ^= 1;

        let result = decrypt_block_payload(&cipher, &encrypted_block);
        assert!(result.is_err());
    }
}
