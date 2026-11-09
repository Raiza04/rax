//! Interactive Read-Eval-Print Loop (REPL) for the `rax` virtual file system.
//!
//! This module provides a continuous interactive shell using `rustyline` with
//! Bash-style tab completion, live syntax highlighting, and inline parameter hints.

use std::borrow::Cow::{self, Borrowed, Owned};
use std::io::Result;

use clap::{Parser, Subcommand};
use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::{CmdKind, Highlighter};
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::validate::Validator;
use rustyline::{CompletionType, Config, Context, Editor, Helper};

use crate::fs::Inode;
use crate::vault::Vault;

/// Autocompletion, syntax highlighting, and inline hinting helper for the `rax` REPL.
pub struct RaxHelper {
    /// In-memory cache of filenames currently present in the vault.
    pub vault_files: Vec<String>,
    /// Native helper to traverse and complete file paths from the host OS.
    host_completer: FilenameCompleter,
    /// Known interactive REPL commands.
    commands: Vec<&'static str>,
}

impl RaxHelper {
    /// Constructs a new `RaxHelper` seeded with the current vault files.
    ///
    /// # Arguments
    /// * `files` - Initial slice of inodes loaded from the vault.
    pub fn new(files: &[Inode]) -> Self {
        Self {
            vault_files: files.iter().map(|f| f.name.clone()).collect(),
            host_completer: FilenameCompleter::new(),
            commands: vec!["ls", "put", "get", "rm", "cat", "edit", "clear", "exit"],
        }
    }

    /// Refreshes the cached vault filenames to maintain parity with the `InodeTable`.
    ///
    /// # Arguments
    /// * `files` - The updated slice of inodes from `vault.table.files`.
    pub fn update_files(&mut self, files: &[Inode]) {
        self.vault_files = files.iter().map(|f| f.name.clone()).collect();
    }

    /// Applies ANSI color codes to an individual token based on its position and role.
    ///
    /// * Position 0 (Command): Bold green if valid, bold red if unknown.
    /// * Position 1+ (Arguments): Cyan for flags (`-`), bold yellow for existing vault files.
    fn colorize_token(&self, token: &str, index: usize) -> String {
        if index == 0 {
            if self.commands.contains(&token) {
                format!("\x1b[1;32m{}\x1b[0m", token)
            } else {
                format!("\x1b[1;31m{}\x1b[0m", token)
            }
        } else if token.starts_with('-') {
            format!("\x1b[36m{}\x1b[0m", token)
        } else if self.vault_files.iter().any(|f| f == token) {
            format!("\x1b[1;33m{}\x1b[0m", token)
        } else {
            token.to_string()
        }
    }
}

impl Completer for RaxHelper {
    type Candidate = Pair;

    /// Evaluates the input line and generates context-aware tab completions.
    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> std::result::Result<(usize, Vec<Pair>), ReadlineError> {
        let line_to_cursor = &line[..pos];
        let mut tokens = line_to_cursor.split_whitespace();
        let first_word = tokens.next();

        let is_first_token = match first_word {
            None => true,
            Some(_) if !line_to_cursor.contains(' ') => true,
            _ => false,
        };

        if is_first_token {
            let prefix = first_word.unwrap_or("");
            let candidates = self
                .commands
                .iter()
                .filter(|cmd| cmd.starts_with(prefix))
                .map(|cmd| Pair {
                    display: cmd.to_string(),
                    replacement: cmd.to_string(),
                })
                .collect();

            return Ok((pos - prefix.len(), candidates));
        }

        let cmd = first_word.unwrap();
        match cmd {
            "put" => self.host_completer.complete(line, pos, ctx),

            "cat" | "edit" | "rm" => {
                let last_token = line_to_cursor.split_whitespace().last().unwrap_or("");
                let (prefix, start) = if line_to_cursor.ends_with(' ') {
                    ("", pos)
                } else {
                    (last_token, pos - last_token.len())
                };

                let candidates = self
                    .vault_files
                    .iter()
                    .filter(|name| name.starts_with(prefix))
                    .map(|name| Pair {
                        display: name.clone(),
                        replacement: name.clone(),
                    })
                    .collect();

                Ok((start, candidates))
            }

            "get" => {
                let tokens_vec: Vec<&str> = line_to_cursor.split_whitespace().collect();
                let ends_with_space = line_to_cursor.ends_with(' ');
                let last_token = tokens_vec.last().copied().unwrap_or("");

                // Determine the active flag preceding the current value
                let active_flag = if ends_with_space {
                    last_token
                } else if tokens_vec.len() >= 2 {
                    tokens_vec[tokens_vec.len() - 2]
                } else {
                    ""
                };

                if !ends_with_space && last_token.starts_with('-') {
                    let flags = ["--file", "--dest"];
                    let candidates = flags
                        .iter()
                        .filter(|f| f.starts_with(last_token))
                        .map(|f| Pair {
                            display: f.to_string(),
                            replacement: f.to_string(),
                        })
                        .collect();
                    return Ok((pos - last_token.len(), candidates));
                }

                if active_flag == "-d" || active_flag == "--dest" {
                    self.host_completer.complete(line, pos, ctx)
                } else {
                    let (prefix, start) = if ends_with_space {
                        ("", pos)
                    } else {
                        (last_token, pos - last_token.len())
                    };

                    let candidates = self
                        .vault_files
                        .iter()
                        .filter(|name| name.starts_with(prefix))
                        .map(|name| Pair {
                            display: name.clone(),
                            replacement: name.clone(),
                        })
                        .collect();

                    Ok((start, candidates))
                }
            }

            _ => Ok((pos, Vec::new())),
        }
    }
}

impl Hinter for RaxHelper {
    type Hint = String;

    /// Provides dimmed inline suggestions and parameter blueprints when the cursor is at the end of the line.
    fn hint(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Option<Self::Hint> {
        if pos < line.len() || line.is_empty() {
            return None;
        }

        // 1. Hint command names while typing the first word
        if !line.contains(' ') {
            return self
                .commands
                .iter()
                .find(|cmd| cmd.starts_with(line) && **cmd != line)
                .map(|cmd| cmd[line.len()..].to_string());
        }

        let mut parts = line.split_whitespace();
        let cmd = parts.next()?;
        let rest: Vec<&str> = parts.collect();

        // 2. Show parameter blueprints immediately after typing `<command> `
        if rest.is_empty() && line.ends_with(' ') {
            return match cmd {
                "put" => Some("<host_path> [--remove]".to_string()),
                "get" => Some("--file <vault_file> --dest <host_path>".to_string()),
                "cat" | "edit" | "rm" => Some("<vault_file>".to_string()),
                _ => None,
            };
        }

        // 3. Hint matching vault filenames while typing an argument for cat, edit, or rm
        if matches!(cmd, "cat" | "edit" | "rm") && rest.len() == 1 && !line.ends_with(' ') {
            let prefix = rest[0];
            return self
                .vault_files
                .iter()
                .find(|name| name.starts_with(prefix) && name.as_str() != prefix)
                .map(|name| name[prefix.len()..].to_string());
        }

        None
    }
}

impl Highlighter for RaxHelper {
    /// Colors the prompt in bold green while allowing `rustyline` to calculate the exact visible width.
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        _default: bool,
    ) -> Cow<'b, str> {
        Owned(format!("\x1b[1;32m{}\x1b[0m", prompt))
    }

    /// Renders inline hints in subtle dark gray.
    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned(format!("\x1b[90m{}\x1b[0m", hint))
    }

    /// Tokenizes the input line while preserving exact whitespace to apply live syntax highlighting.
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        if line.is_empty() {
            return Borrowed(line);
        }

        let mut result = String::with_capacity(line.len() + 32);
        let mut in_word = false;
        let mut word_start = 0;
        let mut word_index = 0;

        for (i, ch) in line.char_indices() {
            if ch.is_whitespace() {
                if in_word {
                    let word = &line[word_start..i];
                    result.push_str(&self.colorize_token(word, word_index));
                    word_index += 1;
                    in_word = false;
                }
                result.push(ch);
            } else if !in_word {
                word_start = i;
                in_word = true;
            }
        }

        if in_word {
            let word = &line[word_start..];
            result.push_str(&self.colorize_token(word, word_index));
        }

        Owned(result)
    }

    /// Forces live re-highlighting on every character insertion or deletion.
    fn highlight_char(&self, _line: &str, _pos: usize, kind: CmdKind) -> bool {
        kind != CmdKind::MoveCursor
    }
}

impl Validator for RaxHelper {}
impl Helper for RaxHelper {}

/// The internal command-line interface for the active REPL session.
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

    /// Edits a text file in RAM using `$VISUAL`, `$EDITOR`, or `nano`.
    Edit {
        /// The exact name of the file to edit.
        file: String,
    },

    /// Clears the terminal screen.
    Clear,
}

/// Starts the interactive shell session, blocking the main thread until the user exits.
pub fn run_shell(vault: &mut Vault) -> Result<()> {
    print!("\x1b[1A\x1b[2K\r\x1b[32mWelcome to the rax shell. Type 'exit' to quit.\x1b[0m\n");

    // Configure Bash-style list completion and prompt threshold
    let config = Config::builder()
        .completion_type(CompletionType::List)
        .completion_prompt_limit(20)
        .build();

    let mut rl =
        Editor::<RaxHelper, DefaultHistory>::with_config(config).map_err(std::io::Error::other)?;
    let helper = RaxHelper::new(&vault.table.files);
    rl.set_helper(Some(helper));

    loop {
        if let Some(h) = rl.helper_mut() {
            h.update_files(&vault.table.files);
        }

        let readline = rl.readline("rax> ");
        match readline {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }

                let _ = rl.add_history_entry(input);

                if input == "exit" {
                    println!("Locking vault...");
                    vault.sync()?;
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
                            match vault.put_file(&path, &remove).and_then(|_| vault.sync()) {
                                Ok(_) => {
                                    println!("File successfully encrypted and stored in vault.")
                                }
                                Err(e) => println!("Error importing file: {}", e),
                            }
                        }
                        Some(ReplCommands::Get { file, dest }) => {
                            match vault.get_file(&file, &dest) {
                                Ok(_) => println!("'{}' successfully extracted.", file),
                                Err(e) => println!("Error extracting file: {}", e),
                            }
                        }
                        Some(ReplCommands::Rm { file }) => {
                            match crate::builtin::rm(vault, &file).and_then(|_| vault.sync()) {
                                Ok(_) => {
                                    println!("File '{}' permanently removed from vault.", file)
                                }
                                Err(e) => println!("Error removing file: {}", e),
                            }
                        }
                        Some(ReplCommands::Cat { file }) => {
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
                            let _ = rl.clear_screen();
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
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                println!("Locking vault...");
                vault.sync()?;
                break;
            }
            Err(err) => {
                println!("Error reading line: {}", err);
                break;
            }
        }
    }
    Ok(())
}
