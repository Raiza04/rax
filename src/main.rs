//! Entry point for the `rax` Encrypted Virtual File System.
//!
//! This module handles the initial command-line arguments directly from the host OS (Bash/Zsh),
//! manages secure password prompts, and orchestrates the transition between one-off
//! host commands (like `create` or `put`) and the interactive `rax` REPL shell.

use clap::{Parser, Subcommand};
use rpassword::prompt_password;
use std::io::Result;

mod builtin;
mod crypto;
mod fs;
mod shell;
mod vault;

use vault::{Vault, get_vault_path};

/// Command-line interface for the host OS.
///
/// Defines the arguments accepted when launching `rax` from a standard terminal.
/// If no subcommand is provided, it defaults to opening the interactive shell.
#[derive(Parser)]
#[command(author = "Raiza", version, about = "Encrypted Virtual File System", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

/// Available subcommands for the host OS terminal.
#[derive(Subcommand)]
enum Commands {
    /// Creates the central vault file.
    Create {
        /// Total size of the vault to allocate in megabytes.
        #[arg(short, long)]
        size: usize,
    },

    /// Imports a file into the vault directly from the host terminal.
    Put {
        /// Absolute or relative path to the file to import.
        path: String,

        /// Securely deletes the original file after successful import.
        #[arg(short, long)]
        remove: bool,
    },
}

/// Securely prompts the user twice for a new password and verifies they match.
///
/// Used during vault creation to ensure the user did not make a typo
/// in their master password.
///
/// # Errors
/// Exits the process immediately if the passwords do not match.
/// Returns an `std::io::Error` if the terminal prompt fails.
fn setup_admin() -> Result<String> {
    let pass1 = prompt_password("Enter a new password: ")?;
    let pass2 = prompt_password("Repeat the password: ")?;

    if pass1 != pass2 {
        println!("Passwords didn`t match. Make sure they are the same. ");
        std::process::exit(1);
    }

    Ok(pass1)
}

/// Core application logic and routing for host-level commands.
///
/// Parses CLI arguments, prompts for passwords as needed, and either executes
/// a single one-off command (`create`, `put`) or hands control over to the
/// interactive `shell::run_shell`.
///
/// # Errors
/// Bubbles up any `std::io::Error` from vault initialization, crypto, or file I/O.
fn run_app() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Create { size }) => {
            let admin_pass = setup_admin()?;

            let vault_path = get_vault_path()?;
            println!(
                "Creating vault at '{}' with a size of {} MB...",
                vault_path.display(),
                size
            );

            Vault::create(&vault_path, *size, &admin_pass)?;
        }

        Some(Commands::Put { path, remove }) => {
            let pass_auth = prompt_password("Enter the admin password: ")?;

            let vault_path = get_vault_path()?;
            let mut vault = Vault::open(&vault_path, &pass_auth)?;

            vault.put_file(path, remove)?;
            vault.sync()?;

            println!("File successfuly encrypted and stored in vault");
        }

        None => {
            let pass_auth = prompt_password("Enter the admin password: ")?;

            let vault_path = get_vault_path()?;
            let mut opened_vault = Vault::open(&vault_path, &pass_auth)?;

            shell::run_shell(&mut opened_vault)?;
        }
    }

    Ok(())
}

/// The main entry point.
///
/// Wraps `run_app()` to provide a clean exit with an error code and a formatted
/// error message if a critical failure bubbles all the way up from the backend.
fn main() {
    if let Err(e) = run_app() {
        eprintln!("Error: {} ", e);
        std::process::exit(1);
    }
}
