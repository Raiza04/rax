use clap::{Parser, Subcommand};
use rpassword::prompt_password;
use std::io::Result;

mod builtin;
mod crypto;
mod fs;
mod shell;
mod vault;

use vault::{Vault, get_vault_path};

#[derive(Parser)]
#[command(author = "Raiza", version, about = "Encrypted Virtual File System", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Creates the central vault file
    Create {
        /// Size of the vault in megabytes
        #[arg(short, long)]
        size: usize,
    },

    ///Puts a file in vault
    Put {
        /// Path to the file to import
        #[arg(short, long)]
        path: String,

        #[arg(short, long)]
        remove: bool,
    },
}

fn setup_admin() -> Result<String> {
    let pass1 = prompt_password("Enter a new password: ")?;
    let pass2 = prompt_password("Repeat the password: ")?;

    if pass1 != pass2 {
        println!("Passwords didn`t match. Make sure they are the same. ");
        std::process::exit(1);
    }

    Ok(pass1)
}

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

fn main() {
    if let Err(e) = run_app() {
        eprintln!("Error: {} ", e);

        std::process::exit(1);
    }
}
