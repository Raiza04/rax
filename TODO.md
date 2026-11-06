
# Roadmap & TODOs

Project backlog and technical milestones for the `rax` Virtual File System.

---

## 🗄️ 1. Storage & Header Architecture Refactoring
- [ ] **Replace header chunks with fixed-size blocks:**
  - [ ] Remove dynamic chunks (`write_encrypted_chunk` / `read_encrypted_chunk`) from the header
  - [ ] Fully transition the header and system metadata to standard 4096-byte blocks
  - [ ] Define predictable, fixed block offsets for system metadata (e.g., Block 0: Superblock/Magic/Salt, Block 1..n: InodeTable)
- [ ] **Dynamically expand vault size (`grow` / `resize` / `add`):**
  - [ ] Implement command to extend an existing `.rax` vault file by a given number of megabytes
  - [ ] Physically append cryptographic noise to the end of the container file
  - [ ] Dynamically resize the bitmap (`free_blocks`) in `InodeTable` and flag newly added blocks as free
  - [ ] Ensure atomic synchronization to safeguard against corruption if resized mid-operation

---

## 🪟 2. Cross Platform & Windows Memory-Safety Alternative
- [ ] **RAM-backed folder alternative for Windows:**
  - [ ] Direct in-memory piping: Keep decrypted files purely in RAM (`Vec<u8>`) for reading and viewing without touching disk
  - [ ] Secure temp files with immediate shredding (when external editors require a physical file path):
    - [ ] Allocate secure temporary paths via the `tempfile` crate
    - [ ] Perform a zero/randomized byte wipe immediately after the external editor process terminates
- [ ] **Platform-specific compatibility:**
  - [ ] Normalize filesystem path separators (`/` vs. `\`) across platforms using `std::path::Path`
  - [ ] Implement editor fallback chain: `%VISUAL%` -> `%EDITOR%` -> `notepad.exe` (Windows) vs. `vi`/`nano` (Unix)
  - [ ] Set up GitHub Actions CI matrix for `ubuntu-latest`, `macos-latest`, and `windows-latest`

---

## 🗂️ 3. Directory Integration in Filesystem
- [ ] **Extend Inode structure:**
  - [ ] Introduce a `FileType` enum (`File`, `Directory`) or an `is_dir: bool` flag inside `Inode`
- [ ] **Hierarchical directory tree:**
  - [ ] Support nested paths (e.g., `/documents/work/notes.txt`) instead of flat filenames
  - [ ] Store child Inode entries and path lookups within directory Inodes
- [ ] **Shell directory commands:**
  - [ ] `mkdir <dir>`: Create a new directory
  - [ ] `cd <dir>`: Change current working directory (with support for `..` and `/`)
  - [ ] `pwd`: Print the current working path in the shell
  - [ ] Support recursive operations for `rm -r` and recursive directory imports via `put`

---

## ⌨️ 4. Tab Completion & Interactive Shell
- [ ] **Custom Rustyline helper:**
  - [ ] Implement a custom `Completer` and `Validator` for the `rax` shell
- [ ] **Command autocompletion:**
  - [ ] Auto-complete all built-in commands (`ls`, `cat`, `put`, `get`, `rm`, `mkdir`, `cd`, `exit`)
- [ ] **Context-aware path completion:**
  - [ ] Internal commands (`cat`, `cd`, `rm`): Suggest files and folders from the current vault directory
  - [ ] External commands (`put`): Suggest matching paths from the host operating system filesystem

---

## 🎨 5. Fine Tuning & Terminal UX
- [ ] **Dynamic REPL prompt:**
  - [ ] Render the active virtual path dynamically (e.g., `rax:~/documents> `)
- [ ] **Table formatting for `ls`:**
  - [ ] Formatted tabular output with columns for type (`d`/`-`), size, creation timestamp, and block counts
- [ ] **Colorized status feedback:**
  - [ ] Consistent terminal styling: green for success, yellow for warnings, red for errors
- [ ] **Progress indicators:**
  - [ ] Integrate the `indicatif` crate for large file transfers and container creation/resizing

---

## ✅ Completed
- [x] AES-256-GCM block and chunk encryption
- [x] Argon2 master key derivation with custom salt
- [x] Basic block allocation and deallocation engine (`InodeTable`)
- [x] Initial interactive shell built with `rustyline` and `clap`
