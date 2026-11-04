use crate::vault::{self, Vault};
use std::io::{Error, ErrorKind, Result};

pub fn ls(vault: &Vault) {
    if vault.table.files.is_empty() {
        println!("(Vault is empty)");
        return;
    }

    println!("{:<30} | {:<10}", "Name", "Size");
    println!("{:-<30}-+-{:-<10}", "", "");

    for inode in vault.table.files.iter() {
        let size_kb = inode.size_bytes.div_ceil(1024);

        println!("{:<30} | {} KB", inode.name, size_kb);
    }
}

/// Removes a file from the vault by deallocating its blocks and removing its Inode.
pub fn rm(vault: &mut Vault, filename: &str) -> Result<()> {
    let index = vault
        .table
        .files
        .iter()
        .position(|f| f.name == filename)
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "File not found in vault"))?;

    let inode = vault.table.files.remove(index);

    vault.table.deallocate(&inode.blocks);

    println!("File '{}' permanently removed from vault.", filename);
    Ok(())
}
