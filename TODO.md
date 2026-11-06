# 🗺️ Roadmap & Architecture Backlog

Project backlog and technical milestones for the `rax` Encrypted Virtual File System, structured by Semantic Versioning impact.

---

## ⚡ Milestone v0.2.0 — Shell UX & Non-Breaking Enhancements
*Focus: Improving interactive usability and container management without breaking backwards compatibility with v0.1.0 `.rax` vaults.*

### ⌨️ 1. Tab Completion & Interactive Shell
- [ ] **Custom Rustyline helper:**
  - [ ] Implement `Completer`, `Hinter`, and `Validator` traits for the `rax>` REPL
- [ ] **Command & context-aware autocompletion:**
  - [ ] Auto-complete built-in commands (`ls`, `cat`, `put`, `get`, `rm`, `edit`, `clear`, `exit`)
  - [ ] Internal commands (`cat`, `edit`, `rm`, `get`): Suggest existing filenames from the active `InodeTable`
  - [ ] External commands (`put`): Suggest matching file paths from the host OS filesystem

### 🎨 2. Terminal UI & Feedback Polish
- [ ] **Enhanced `ls` formatting:**
  - [ ] Display human-readable creation timestamps, exact byte/KB/MB sizes, and occupied block counts
  - [ ] Add vault capacity summary footer (e.g., `Used: 12.4 MB / 50.0 MB [24%]`)
- [ ] **Visual indicators:**
  - [ ] Consistent ANSI color scheme (green for success, yellow for warnings, red for errors)
  - [ ] Integrate `indicatif` progress bars for large file imports/exports and initial vault creation

### 📈 3. Dynamic Vault Resizing (`grow`)
- [ ] **Expand existing containers in-place:**
  - [ ] Implement `grow <size_mb>` command to append cryptographic RNG noise to the end of `vault.rax`
  - [ ] Extend the `free_blocks` bitmap in `InodeTable` and sync atomically

---

## 🚀 Milestone v2.0.0 — Core VFS Overhaul & Plausible Deniability
*Focus: Major breaking changes to the binary container layout, hierarchical filesystem structures, hidden volumes, and cross-platform memory locking.*

### 🕵️ 1. Hidden Vaults (Plausible Deniability)
- [ ] **Dual-password architecture:**
  - [ ] Support initializing a secondary, hidden `InodeTable` within the reserved 1 MB system lobby using a second master password
  - [ ] Route login dynamically: Password A mounts the outer decoy vault; Password B mounts the hidden inner vault
- [ ] **Cryptographic indistinguishability:**
  - [ ] Ensure unused lobby blocks and unallocated data blocks are indistinguishable from uniform AES-GCM ciphertext
  - [ ] Allocate hidden vault blocks from the upper boundary of the container to minimize accidental overwrites when using the outer decoy vault

### 🗄️ 2. Fixed-Block Header Architecture
- [ ] **Eliminate variable-sized metadata chunks:**
  - [ ] Deprecate `write_encrypted_chunk` and `read_encrypted_chunk` in favor of strict 4096-byte block alignment across the entire file
- [ ] **Deterministic Lobby layout (Blocks `0..255`):**
  - [ ] **Block 0 (Superblock):** Magic bytes (`RAX2`), format version, and cryptographic salts
  - [ ] **Blocks 1..127:** Outer Vault Auth & spanned `InodeTable` blocks
  - [ ] **Blocks 128..255:** Hidden Vault Auth & spanned `InodeTable` blocks (or pure RNG noise if unused)

### 🗂️ 3. Hierarchical Directory Tree (`Inode` v2)
- [ ] **Redesigned `Inode` structure:**
  - [ ] Add unique `inode_id: u64` and `parent_id: Option<u64>`
  - [ ] Introduce `FileType` enum (`File`, `Directory`)
- [ ] **Path resolution & directory commands:**
  - [ ] Support nested virtual paths (e.g., `/docs/keys/backup.txt`) and relative navigation (`.` and `..`)
  - [ ] Implement `mkdir <path>`, `cd <path>`, `pwd`, and `mv <src> <dest>`
  - [ ] Support recursive directory imports (`put -r`) and deletions (`rm -r`)
  - [ ] Update REPL prompt to reflect the active directory (`rax:/docs/keys> `)

### 🛡️ 4. Cross-Platform RAM Safety & Built-in Editor
- [ ] **OS-level memory locking (Swap protection):**
  - [ ] Lock sensitive plaintext buffers in physical RAM using `mlock` (Linux/macOS) and `VirtualLock` (Windows) so the OS never writes secrets to disk swap / `pagefile.sys`
- [ ] **Zero-disk cross-platform editing:**
  - [ ] Build an integrated, lightweight TUI text editor (via `ratatui` / `crossterm`) to edit files purely in locked Rust RAM on Windows and macOS without relying on `/dev/shm` or touching the host SSD
  - [ ] Keep `/dev/shm` external `$EDITOR` integration as an opt-in power feature on Linux

---

## ✅ Completed (v0.1.0 MVP)
- [x] AES-256-GCM 4096-byte block encryption & tamper-proof MAC verification
- [x] Argon2id memory-hard master key derivation
- [x] Custom block allocator and `bincode`-serialized `InodeTable`
- [x] Interactive `rustyline` REPL with dual `clap` CLI/Shell parsers
- [x] Linux `/dev/shm` RAM-disk integration for external terminal editors (`edit`)
- [x] Strict volatile RAM wiping via `zeroize` across all plaintext buffers and keys
- [x] Unit & negative edge-case test suite
