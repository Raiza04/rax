use aes_gcm::{
    Aes256Gcm, Key as AesKey, Nonce as AesNonce,
    aead::{Aead, KeyInit},
};
use argon2::Argon2;

use rand::RngExt;
use std::fs::File;
use std::io::{Error, ErrorKind, Read, Result, Write};

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

/// Encrypts exactly 4068 bytes of payload into a full 4096-byte physical block.
/// The resulting block structure: [Nonce (12)] + [Ciphertext + MAC Tag (4084)]
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

/// Decrypts a full 4096-byte physical block and returns the 4068-byte original payload.
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

    //4084 - 16 (MAC Tag) = 4068
    let mut result = [0u8; 4068];
    result.copy_from_slice(&payloadtext);

    Ok(result)
}

/// Helper: Encrypts data and writes the packet (Size + Nonce + Ciphertext) to the file
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

/// Helper: Reads a packet (Size + Nonce + Ciphertext) from the file and decrypts it
pub fn read_encrypted_chunk(file: &mut File, cipher: &Aes256Gcm) -> Result<Vec<u8>> {
    // 1. Read the size of the incoming ciphertext (8 Bytes)
    let mut size_bytes = [0u8; 8];
    file.read_exact(&mut size_bytes)?;
    let size = u64::from_le_bytes(size_bytes) as usize;

    // 2. Read the nonce specifically used for this chunk (12 Bytes)
    let mut nonce_bytes = [0u8; 12];
    file.read_exact(&mut nonce_bytes)?;
    let aes_nonce = AesNonce::from(nonce_bytes);

    // 3. Read the actual ciphertext
    let mut ciphertext = vec![0u8; size];
    file.read_exact(&mut ciphertext)?;

    // 4. Decrypt the data
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
