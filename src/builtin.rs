//! Built-in command implementations for the interactive `rax` shell.
//!
//! This module acts as the bridge between the user's terminal input
//! and the low-level virtual file system operations. It handles output formatting
//! (like tables for `ls`) and safety guarantees (like UTF-8 validation for `cat`).

use crate::vault::Vault;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::io::{Error, ErrorKind, Result};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Lists all files currently stored in the vault, formatted as an ASCII table.
///
/// Iterates over the `InodeTable` and prints the name and human-readable
/// size (in KB) for each file. Displays a fallback message if the vault is empty.
///
/// # Arguments
/// * `vault` - A reference to the currently open vault.
pub fn ls(vault: &Vault) {
    if vault.table.files.is_empty() {
        println!("(Vault is empty)");
        return;
    }

    println!("{:<30} | {:<10}", "Name", "Size");
    println!("{:-<30}-+-{:-<10}", "", "");

    for inode in vault.table.files.iter() {
        // div_ceil rounds up, so a 1-byte file shows as 1 KB instead of 0 KB
        let size_kb = inode.size_bytes.div_ceil(1024);

        println!("{:<30} | {} KB", inode.name, size_kb);
    }
}

/// Permanently removes a file from the vault.
///
/// Locates the file in the `InodeTable`, removes its entry, and deallocates
/// its physical blocks so they can be overwritten by future imports.
///
/// # Arguments
/// * `vault` - A mutable reference to the open vault.
/// * `filename` - The exact name of the file to remove.
///
/// # Errors
/// Returns an `std::io::Error` of kind `NotFound` if the file does not exist.
pub fn rm(vault: &mut Vault, filename: &str) -> Result<()> {
    let index = vault
        .table
        .files
        .iter()
        .position(|f| f.name == filename)
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "File not found in vault"))?;

    let inode = vault.table.files.remove(index);

    vault.table.deallocate(&inode.blocks);

    Ok(())
}

/// Reads a file directly into RAM and prints its contents to the terminal.
///
/// Protects the shell by validating that the decrypted bytes are valid UTF-8
/// text before printing. This prevents raw binary data (like images) from
/// corrupting the terminal formatting.
///
/// # Arguments
/// * `vault` - A mutable reference to the open vault.
/// * `filename` - The name of the file to read.
///
/// # Errors
/// Returns an error if the file is not found, cannot be decrypted, or contains
/// invalid UTF-8 (binary data).
pub fn cat(vault: &mut Vault, filename: &str) -> Result<()> {
    // 1. Fetch the decrypted raw bytes entirely into RAM
    let raw_bytes = vault.read_file_to_memory(filename)?;

    // 2. Strict UTF-8 validation: Refuse to print if it's a binary file
    let text = std::str::from_utf8(&raw_bytes).map_err(|_| {
        Error::new(
            ErrorKind::InvalidData,
            "File contains binary data (not valid UTF-8) and cannot be printed safely.",
        )
    })?;

    // 3. Print the validated text to the terminal
    println!("{}", text);

    Ok(())
}

/// Extracts a text file to a RAM disk, opens it in the user's preferred terminal editor,
/// and securely re-imports it into the vault upon completion.
///
/// This workflow leverages `/dev/shm` (Shared Memory) on Linux/Unix systems to ensure
/// that the decrypted file data is held strictly in RAM and never written to the physical SSD.
///
/// # Arguments
/// * `vault` - A mutable reference to the open vault.
/// * `filename` - The name of the file to edit.
///
/// # Errors
/// Returns an error if the file cannot be extracted, if the RAM disk is unavailable,
/// or if the external editor process fails to spawn.
pub fn edit(vault: &mut Vault, filename: &str) -> Result<()> {
    let raw_bytes = vault.read_file_to_memory(filename)?;

    // Prevent rendering binary gibberish in the terminal editor
    std::str::from_utf8(&raw_bytes).map_err(|_| {
        Error::new(
            ErrorKind::InvalidData,
            "File contains binary data. Editing is only supported for text files.",
        )
    })?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros();
    let tmp_path = format!("/dev/shm/.rax_tmp_{}_{}", filename, timestamp);

    std::fs::write(&tmp_path, &raw_bytes)?;

    let editor = env::var("VISUAL")
        .or_else(|_| env::var("EDITOR"))
        .unwrap_or_else(|_| "nano".to_string());

    let mut child = Command::new(&editor).arg(&tmp_path).spawn().map_err(|e| {
        Error::new(
            ErrorKind::NotFound,
            format!("Failed to start editor '{}': {}", editor, e),
        )
    })?;

    // Blocks the REPL until the external editor process terminates
    let status = child.wait()?;

    if status.success() {
        crate::builtin::rm(vault, filename)?;
        vault.put_file(&tmp_path, &false)?;

        if let Some(inode) = vault.table.files.last_mut() {
            inode.name = filename.to_string();
        }

        println!("File '{}' successfully updated.", filename);
    } else {
        println!("Editor closed with an error. No changes were saved.");
    }

    // Secure cleanup: manually zero-out the volatile RAM disk file before dropping it
    if let Ok(mut file) = OpenOptions::new().write(true).open(&tmp_path) {
        let size = std::fs::metadata(&tmp_path)?.len() as usize;
        let zeros = vec![0u8; size];
        let _ = file.write_all(&zeros);
        let _ = file.sync_all();
    }

    let _ = std::fs::remove_file(&tmp_path);

    Ok(())
}
