//! Built-in command implementations for the interactive `rax` shell.
//!
//! This module acts as the bridge between the user's terminal input
//! and the low-level virtual file system operations. It handles output formatting
//! (like tables for `ls`) and safety guarantees (like UTF-8 validation for `cat`).

use crate::ui;
use crate::vault::Vault;
use chrono::{DateTime, Local};
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::io::{Error, ErrorKind, Result};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use zeroize::Zeroize;

/// Lists all files currently stored in the vault, formatted as an ASCII table.
///
/// Iterates over the `InodeTable` and prints the name and human-readable
/// size (in KB) for each file. Displays a fallback message if the vault is empty.
///
/// # Arguments
/// * `vault` - A reference to the currently open vault.
/// * `detail` - If true, additional metadata (like timestamps) will be displayed for each file.
pub fn ls(vault: &Vault, detail: bool) {
    if vault.table.files.is_empty() {
        ui::info("(Vault is empty)");
        return;
    }

    if !detail {
        for inode in vault.table.files.iter() {
            println!("{}", inode.name);
        }
        return;
    }

    println!("{:<30} | {:<10} | {:<20}", "Name", "Size", "Created");
    println!("{:-<30}-+-{:-<10}-+-{:-<20}", "", "", "");

    for inode in vault.table.files.iter() {
        let dt_uts =
            DateTime::from_timestamp(inode.created_at as i64, 0).expect("Invalid timestamps");
        let dt_local: DateTime<Local> = dt_uts.with_timezone(&Local);

        println!(
            "{:<30} | {:<10} | {} ",
            inode.name,
            format_size(inode.size_bytes),
            dt_local.format("%Y-%m-%d %H:%M:%S")
        );
    }

    let total_data_blocks = vault.table.free_blocks.len();
    let total_data_bytes = total_data_blocks * 4096;

    let allocated_blocks = vault.table.free_blocks.iter().filter(|&&b| b).count();
    let allocated_bytes = allocated_blocks * 4096;

    let relative_allocation = (allocated_bytes as f64 / total_data_bytes as f64) * 100.0;

    println!("{:-<30}-+-{:-<10}-+-{:-<20}", "", "", "");

    println!(
        "Vault capacity: {} / {} used [{:.2}%]",
        format_size(allocated_bytes as u64),
        format_size(total_data_bytes as u64),
        relative_allocation
    );
}

/// formats a file size in bytes into a human-readable string with appropriate units (B, KB, MB,
/// GB).
///
/// # Arguments
/// * `size_bytes` - The size of the file in bytes.
///
/// # Returns
/// returns a `String` representing the formatted size.
fn format_size(size_bytes: u64) -> String {
    if size_bytes < 1024 {
        format!("{} B", size_bytes)
    } else if size_bytes < 1024 * 1024 {
        format!("{:.2} KB", size_bytes as f64 / 1024.0)
    } else if size_bytes < 1024 * 1024 * 1024 {
        format!("{:.2} MB", size_bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", size_bytes as f64 / (1024.0 * 1024.0 * 1024.0))
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
    let mut raw_bytes = vault.read_file_to_memory(filename)?;

    match std::str::from_utf8(&raw_bytes) {
        Ok(text) => {
            println!("{}", text);
            raw_bytes.zeroize();
            Ok(())
        }
        Err(_) => {
            raw_bytes.zeroize();
            Err(Error::new(
                ErrorKind::InvalidData,
                "File contains binary data (not valid UTF-8) and cannot be printed safely.",
            ))
        }
    }
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
    let mut raw_bytes = vault.read_file_to_memory(filename)?;

    if std::str::from_utf8(&raw_bytes).is_err() {
        raw_bytes.zeroize();
        return Err(Error::new(
            ErrorKind::InvalidData,
            "File contains binary data. Editing is only supported for text files.",
        ));
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros();
    let tmp_path = format!("/dev/shm/.rax_tmp_{}_{}", filename, timestamp);

    let write_res = std::fs::write(&tmp_path, &raw_bytes);
    raw_bytes.zeroize();
    write_res?;

    let editor = env::var("VISUAL")
        .or_else(|_| env::var("EDITOR"))
        .unwrap_or_else(|_| "nano".to_string());

    // Run editor and capture result without early-returning before cleanup
    let edit_result = (|| -> Result<()> {
        let mut child = Command::new(&editor).arg(&tmp_path).spawn().map_err(|e| {
            Error::new(
                ErrorKind::NotFound,
                format!("Failed to start editor '{}': {}", editor, e),
            )
        })?;

        let status = child.wait()?;

        if status.success() {
            crate::builtin::rm(vault, filename)?;
            vault.put_file(&tmp_path, &false)?;

            if let Some(inode) = vault.table.files.last_mut() {
                inode.name = filename.to_string();
            }

            vault.sync()?;
        } else {
            ui::error("Editor closed with an error. No changes were saved.");
        }
        Ok(())
    })();

    if let Ok(mut file) = OpenOptions::new().write(true).open(&tmp_path)
        && let Ok(meta) = file.metadata()
    {
        let zeros = vec![0u8; meta.len() as usize];
        let _ = file.write_all(&zeros);
        let _ = file.sync_all();
    }

    let _ = std::fs::remove_file(&tmp_path);

    edit_result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size_units() {
        assert_eq!(format_size(52), "52 B");
        assert_eq!(format_size(2048), "2.00 KB");
        assert_eq!(format_size(8 * 1024 * 1024), "8.00 MB");
    }
}
