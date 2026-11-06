//! Interactive Read-Eval-Print Loop (REPL) for the `rax` virtual file system.
//!
//! This module provides a continuous interactive shell using `rustyline` for
//! command history and line editing. It leverages `clap`'s `try_parse_from`
//! to safely parse in-memory string vectors, ensuring that invalid commands
//! print helpful error messages without crashing the shell environment.

use rustyline::DefaultEditor;
use std::io::Result;

use crate::vault::Vault;
use clap::{Parser, Subcommand};

/// The internal command-line interface for the active REPL session.
///
/// Uses a blank name and disables version flags so that `clap`'s auto-generated
/// error messages fit naturally into the custom `rax>` prompt context.
#[derive(Parser)]
#[command(name = "", disable_version_flag = true)]
struct ReplCli {
    #[command(subcommand)]
    command: Option<ReplCommands>,
}

/// Defines all valid commands available to the user inside the opened vault.
#[derive(Subcommand)]
enum ReplCommands {
    /// Lists all files currently stored in the vault, displaying names and sizes.
    Ls,

    /// Imports a plaintext file from the host OS into the encrypted vault.
    Put {
        /// The absolute or relative path to the file on the host OS.
        path: String,

        /// If set, securely deletes the original file after a successful import.
        #[arg(short, long)]
        remove: bool,
    },

    /// Extracts a file from the vault and saves it decrypted to the host OS.
    Get {
        /// The exact name of the file inside the vault.
        #[arg(short, long)]
        file: String,

        /// The destination path on the host OS where the file should be extracted.
        #[arg(short, long)]
        dest: String,
    },

    /// Permanently deletes a file from the vault and frees up its physical blocks.
    Rm {
        /// The exact name of the file to remove from the vault.
        file: String,
    },

    /// Reads a file from the vault directly into RAM and prints it to the terminal.
    Cat {
        /// The exact name of the text file inside the vault.
        file: String,
    },

    /// Edit a file with the standard editor `$EDITOR` from env
    Edit {
        /// The exact name of the file to edit
        file: String,
    },

    Clear,
}

/// Starts the interactive shell session, blocking the main thread until the user exits.
///
/// Handles graceful shutdowns (e.g., Ctrl+C or `exit`) by ensuring the vault's
/// metadata is synchronized to disk before terminating.
///
/// # Arguments
/// * `vault` - A mutable reference to the initialized and authenticated `Vault`.
///
/// # Errors
/// Returns an error if the terminal editor fails to initialize or if a critical
/// sync operation fails during shutdown.
pub fn run_shell(vault: &mut Vault) -> Result<()> {
    print!("\x1b[1A\x1b[2K\r\x1b[32mWelcome to the rax shell. Type 'exit' to quit.\x1b[0m\n");
    let mut rl = DefaultEditor::new().map_err(std::io::Error::other)?;

    loop {
        let readline = rl.readline("rax> ");
        match readline {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }

                // Handle the manual exit command to trigger a graceful shutdown
                if input == "exit" {
                    println!("Locking vault...");
                    vault.sync().unwrap();
                    break;
                }

                // Prepend a dummy program name so `clap` parses the arguments correctly
                let mut clap_args = vec![""];
                clap_args.extend(input.split_whitespace());

                // Parse the in-memory string vector using clap
                match ReplCli::try_parse_from(clap_args) {
                    Ok(cli) => match cli.command {
                        Some(ReplCommands::Ls) => {
                            crate::builtin::ls(vault);
                        }
                        Some(ReplCommands::Put { path, remove }) => {
                            if let Err(e) = vault.put_file(&path, &remove) {
                                println!("Error importing file: {}", e);
                            }
                            println!("File successfuly encrypted and stored in vault");
                        }
                        Some(ReplCommands::Get { file, dest }) => {
                            // Safely extract the file, printing errors instead of crashing
                            if let Err(e) = vault.get_file(&file, &dest) {
                                println!("Error extracting file: {}", e);
                            }
                            println!("{} successfuly extracted", file);
                        }
                        Some(ReplCommands::Rm { file }) => {
                            // Safely remove the file, printing errors instead of crashing
                            if let Err(e) = crate::builtin::rm(vault, &file) {
                                println!("Error removing file: {}", e);
                            }
                            println!("File '{}' permanently removed from vault.", file);
                        }
                        Some(ReplCommands::Cat { file }) => {
                            // Safely read and print the file to the terminal
                            if let Err(e) = crate::builtin::cat(vault, &file) {
                                println!("Error reading file: {}", e);
                            }
                        }
                        Some(ReplCommands::Edit { file }) => {
                            if let Err(e) = crate::builtin::edit(vault, &file) {
                                println!("Error editing file: {}", e);
                            }
                        }
                        Some(ReplCommands::Clear) => {
                            print!("\x1B[2J\x1B[1;1H");
                        }
                        None => {
                            println!("Command not found");
                        }
                    },
                    Err(e) => {
                        // Clap automatically generates beautiful, colored error messages
                        // (e.g., missing arguments, unknown flags) and prints them here.
                        println!("{}", e);
                    }
                }
            }
            Err(_) => {
                // Catches Ctrl+C or Ctrl+D and ensures the vault is safely locked
                println!("Locking vault...");
                vault.sync().unwrap();
                break;
            }
        }
    }
    Ok(())
}
