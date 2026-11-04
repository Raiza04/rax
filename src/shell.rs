use rustyline::DefaultEditor;
use std::io::Result;

use crate::vault::Vault;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "", disable_version_flag = true)]
struct ReplCli {
    #[command(subcommand)]
    command: Option<ReplCommands>,
}

#[derive(Subcommand)]
enum ReplCommands {
    /// Lists all files in the vault
    Ls,

    /// Imports a file from the host OS
    Put {
        #[arg(short, long)]
        path: String,

        #[arg(short, long)]
        remove: bool,
    },

    /// Extracts a file from the vault
    Get {
        #[arg(short, long)]
        file: String,

        #[arg(short, long)]
        dest: String,
    },

    /// Removes a file permanently from the vault
    Rm {
        #[arg(short, long)]
        file: String,
    },
}

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

                if input == "exit" {
                    println!("Locking vault...");
                    vault.sync().unwrap();
                    break;
                }

                let mut clap_args = vec![""];
                clap_args.extend(input.split_whitespace());

                match ReplCli::try_parse_from(clap_args) {
                    Ok(cli) => match cli.command {
                        Some(ReplCommands::Ls) => {
                            crate::builtin::ls(vault);
                        }
                        Some(ReplCommands::Put { path, remove }) => {
                            if let Err(e) = vault.put_file(&path, &remove) {
                                println!("Error importing file: {}", e);
                            }
                        }
                        Some(ReplCommands::Get { file, dest }) => {
                            //if let Err(e) = vault.get_file(&file, &dest) {
                            //    println!("Error extracting file: {}", e);
                            //}
                        }
                        Some(ReplCommands::Rm { file }) => {
                            crate::builtin::rm(vault, &file)?;
                            println!("(rm not implemented yet)");
                        }
                        None => {
                            println!("Command not found");
                        }
                    },
                    Err(e) => {
                        println!("{}", e);
                    }
                }
            }
            Err(_) => {
                println!("Locking vault...");
                vault.sync().unwrap();
                break;
            }
        }
    }
    Ok(())
}
