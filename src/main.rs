//! Entry point for the `rax` Encrypted Virtual File System.
//!
//! This module handles the initial command-line arguments directly from the host OS (Bash/Zsh),
//! manages secure password prompts, and orchestrates the transition between one-off
//! host commands (like `create` or `put`) and the interactive `rax` REPL shell.

use clap::{Parser, Subcommand};
use rpassword::prompt_password;
use std::io::Result;
use std::path::PathBuf;
use zeroize::Zeroize;

mod builtin;
mod crypto;
mod fs;
mod shell;
mod ui;
mod vault;

use vault::{Vault, get_vault_path, import_vault};

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

    /// Makes the vault file larger by allocating additional space.
    Grow { size: usize },

    Import {
        path: PathBuf,

        #[arg(long, conflicts_with = "backup")]
        force: bool,

        #[arg(long, conflicts_with = "force")]
        backup: bool,
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
    let mut pass1 = prompt_password("Enter a new password: ")?;
    let mut pass2 = prompt_password("Repeat the password: ")?;

    if pass1 != pass2 {
        pass1.zeroize();
        pass2.zeroize();
        println!("Passwords didn't match. Make sure they are the same.");
        std::process::exit(1);
    }

    pass2.zeroize();
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
            let mut admin_pass = setup_admin()?;
            let vault_path = get_vault_path()?;
            ui::info(&format!(
                "Creating vault at '{}' with a size of {} MB...",
                vault_path.display(),
                size
            ));

            let res = Vault::create(&vault_path, *size, &admin_pass);
            admin_pass.zeroize();
            res?;
            ui::success("Vault successfully created");
        }

        Some(Commands::Put { path, remove }) => {
            let mut vault = check_login()?;
            ui::info(&format!("Importing file '{}' into vault...", path));

            vault.put_file(path, remove)?;
            vault.sync()?;

            ui::success("File successfully encrypted and stored in vault.");
        }
        Some(Commands::Grow { size }) => {
            let mut vault = check_login()?;

            ui::info(&format!("Growing vault by {} MB...", size));
            vault.grow(*size)?;

            ui::info("Vault successfully extended");
        }
        Some(Commands::Import {
            path,
            force,
            backup,
        }) => {
            ui::info(&format!("Importing vault from '{}'...", path.display()));
            let target = get_vault_path()?;
            import_vault(path, &target, *force, *backup)?;
            ui::success(&format!(
                "Vault successfully imported to '{}'",
                target.display()
            ));
        }
        None => {
            let mut vault = check_login()?;
            ui::info("Entering interactive shell. Type 'help' for commands, or 'exit' to quit.");
            shell::run_shell(&mut vault)?;
        }
    }

    Ok(())
}

fn check_login() -> Result<Vault> {
    let vault_path = get_vault_path()?;
    let mut pass_auth = prompt_password("Enter the admin password: ")?;
    let vault = Vault::open(&vault_path, &pass_auth);
    pass_auth.zeroize();
    let res_vault = vault?;

    Ok(res_vault)
}

/// The main entry point.
///
/// Wraps `run_app()` to provide a clean exit with an error code and a formatted
/// error message if a critical failure bubbles all the way up from the backend.
fn main() {
    if let Err(e) = run_app() {
        ui::error(&e.to_string());
        std::process::exit(1);
    }
}
