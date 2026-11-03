use crate::Vault;
use rustyline::DefaultEditor;
use std::io::Result;

pub struct Shell;

impl Shell {
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

                    // String spliting on all whitespace
                    let mut parts = input.split_whitespace();

                    // first word is the command
                    let command = parts.next().unwrap_or("");

                    // all the strings following are args of the command
                    let args: Vec<&str> = parts.collect();

                    match command {
                        "exit" => {
                            println!("Locking vault...");
                            vault.sync()?;
                            break;
                        }
                        "ls" => {
                            if args.contains(&"-h") || args.contains(&"--help") {
                                println!("Usage: ls (Lists all files in the vault)");
                            } else {
                                println!("(ls is not implemented yet. Files will appear here)");
                            }
                        }
                        "import" => {
                            if args.is_empty() {
                                println!("Error: Missing file path. Usage: import <path>");
                            } else {
                                println!("(Will import file: {})", args[0]);
                            }
                        }
                        _ => {
                            println!("Command not found: {}", command);
                        }
                    }
                }
                Err(_) => {
                    println!("Locking vault...");
                    break;
                }
            }
        }
        Ok(())
    }
}
