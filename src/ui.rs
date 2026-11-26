//! Terminal UI utilities, semantic status messages, and progress indicators.

use indicatif::{ProgressBar, ProgressStyle};

use std::fmt::Display;

pub fn red(text: impl Display) -> String {
    format!("\x1b[1;31m{}\x1b[0m", text)
}

pub fn green(text: impl Display) -> String {
    format!("\x1b[1;32m{}\x1b[0m", text)
}

pub fn yellow(text: impl Display) -> String {
    format!("\x1b[1;33m{}\x1b[0m", text)
}

pub fn cyan(text: impl Display) -> String {
    format!("\x1b[1;36m{}\x1b[0m", text)
}

pub fn _dim(text: impl Display) -> String {
    format!("\x1b[90m{}\x1b[0m", text)
}

pub fn _bold(text: impl Display) -> String {
    format!("\x1b[1m{}\x1b[0m", text)
}

/// Prints a green success message (`✔`).
pub fn success(msg: &str) {
    println!("\x1b[1;32m✔\x1b[0m {}", msg);
}

/// Prints a yellow warning message (`⚠`).
pub fn warn(msg: &str) {
    println!("\x1b[1;33m⚠ Warning:\x1b[0m {}", msg);
}

/// Prints a cyan informational message (`ℹ`).
pub fn info(msg: &str) {
    println!("\x1b[1;36mInfo:\x1b[0m {}", msg);
}

/// Prints a red error message (`✖`) to stderr.
pub fn error(msg: &str) {
    eprintln!("\x1b[1;31m✖ Error:\x1b[0m {}", msg);
}

/// Creates a styled byte-based progress bar for long-running I/O operations.
///
/// Automatically returns a hidden bar during unit tests (`cfg!(test)`) to keep
/// test output deterministic and free of ANSI control characters.
pub fn create_progress_bar(total_bytes: u64, label: &str) -> ProgressBar {
    if cfg!(test) {
        return ProgressBar::hidden();
    }

    let pb = ProgressBar::new(total_bytes);
    let style = ProgressStyle::with_template(
        "{msg:.cyan.bold} [{bar:35.green/black}] {bytes}/{total_bytes} ({bytes_per_sec}, ETA {eta})",
    )
    .expect("valid indicatif template")
    .progress_chars("█▓░");

    pb.set_style(style);
    pb.set_message(label.to_string());
    pb
}
