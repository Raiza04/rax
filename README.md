<div align="center">

# 🔐 `rax` — Encrypted Virtual File System

**A zero-trace, user-space encrypted container and interactive shell written in Rust.**

[![CI](https://github.com/Raiza04/rax/actions/workflows/ci.yml/badge.svg)](https://github.com/Raiza04/rax/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Raiza04/rax?logo=github&color=brightgreen)](https://github.com/Raiza04/rax/releases)
[![Docs](https://img.shields.io/badge/docs-GitHub%20Pages-blue?logo=rust)](https://Raiza04.github.io/rax/)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg?logo=rust)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-Linux-lightgrey?logo=linux)]()

[![Encryption](https://img.shields.io/badge/encryption-AES--256--GCM-8A2BE2.svg)](https://en.wikipedia.org/wiki/Galois/Counter_Mode)
[![KDF](https://img.shields.io/badge/KDF-Argon2id-00599C.svg)](https://en.wikipedia.org/wiki/Argon2)

[Features](#-key-features) •
[Why rax?](#why-rax) •
[Installation](#-installation) •
[Usage](#-usage) •
[Architecture](#storage-architecture) •
[Limitations](#limitations) •
[API Docs](https://Raiza04.github.io/rax/)

</div>

---

## 🛡️ Overview

`rax` is a lightweight, portable **Encrypted Virtual File System (VFS)** designed to protect sensitive text files, credentials, and documents against SSD wear-leveling leaks and volatile memory remnants.

Unlike traditional disk encryption tools that mount an entire filesystem into the host OS kernel—exposing decrypted files to background indexers, swap files, and OS telemetry—`rax` operates strictly in **isolated user-space**. You interact with your encrypted container through an integrated, memory-safe Read-Eval-Print Loop (REPL) shell (`rax>`).

---

## ✨ Key Features

### 🔒 Security
* **Authenticated Block Cryptography:** Uses **AES-256-GCM** mapped to a fixed `4096`-byte physical block grid (`12`-byte CSPRNG Nonce + `4068`-byte payload + `16`-byte Poly1305 MAC tag). Every block is cryptographically authenticated to detect bit-rot or tampering immediately.
* **Memory-Hard Key Derivation:** Master keys are derived on the fly via **Argon2id** paired with a 32-byte per-vault cryptographic salt, defending against GPU-accelerated brute-force attacks.
* **Volatile RAM Wiping (`zeroize`):** All plaintext file buffers, derived key slices, and password strings are explicitly overwritten with zeros in RAM using volatile memory operations before going out of scope.
* **Zero-SSD-Trace Editing (`/dev/shm`):** When editing a file via `edit`, `rax` extracts the decrypted stream exclusively into Linux Shared Memory (`/dev/shm` RAM disk), launches your `$VISUAL` / `$EDITOR` (or `nano`), re-encrypts the updated file on exit, and zeros out the volatile buffer—never touching physical NAND flash.
* **Cryptographic Noise Initialization:** Newly created and expanded `.rax` containers are filled with CSPRNG noise from byte zero to mask actual file allocation boundaries.

### ⚡ Shell UX & Vault Management
* **Smart Interactive REPL (`rustyline`):** Context-aware Bash-style **tab completion** (commands, vault filenames, and host OS paths), **live syntax highlighting**, and dimmed **inline parameter hints**.
* **Dynamic In-Place Resizing (`grow`):** Expand an existing vault container at any time by appending cryptographic noise and atomically extending the block allocation table—with built-in rollback on I/O errors.
* **Safe Vault Portability (`import`):** Import external `.rax` containers into your local data directory with `RAX1` header validation, collision protection, and optional timestamped backups (`--backup`).
* **Rich Terminal Feedback:** Detailed `ls -d` view with human-readable sizes, creation timestamps, and capacity usage, paired with semantic ANSI status indicators and `indicatif` progress bars for heavy I/O operations.

---

<a name="why-rax"></a>
## ⚖️ Why `rax` Instead of VeraCrypt?

| Feature / Threat Vector | `rax` (User-Space VFS) | VeraCrypt / LUKS (Kernel Mount) |
| :--- | :--- | :--- |
| **Root / `sudo` Privileges** | **Not required** (runs in user-space) | **Required** to mount virtual block devices |
| **OS Indexing & Telemetry** | **Immune** (OS cannot see inside the open vault) | **Exposed** (Spotlight, Windows Search, & thumbs index mounted drives) |
| **SSD Wear-Leveling Leaks** | **Prevented** (`cat` and `edit` stay strictly in RAM) | **Risk via Temp Files** (editors often write `.swp` files to host disk) |
| **Explicit RAM Zeroization** | **Enforced** after every single command | Keys stay mounted in kernel memory until dismount |
| **Workflow** | Instant CLI commands & smart `rax>` shell | Heavy GUI or multi-step loop-device mounting |

---

## 📦 Installation

### 1. Quick Install (One-Liner Binary for Linux x86_64)
Downloads the latest precompiled release binary, verifies its SHA-256 checksum, and installs `rax` globally into `/usr/local/bin`:

```bash
curl -sSL https://raw.githubusercontent.com/Raiza04/rax/main/install.sh | bash
```

### 2. Install via Cargo (One-Liner from Source)
If you have the Rust toolchain installed, Cargo can compile and place the binary directly into `~/.cargo/bin/rax`:

```bash
cargo install --git https://github.com/Raiza04/rax.git
```

### 3. Manual Build from Source
```bash
git clone https://github.com/Raiza04/rax.git
cd rax
cargo build --release

# Install binary globally so you can run 'rax' from anywhere
sudo install -m 755 target/release/rax /usr/local/bin/rax
```

---

## 🚀 Usage

### 1. Host Terminal Commands (Bash / Zsh)
Create or import a vault (stored at `~/.local/share/rax/vault.rax` by default), resize it, or import files directly from your host shell:

```bash
# Create a new 50 MB encrypted vault
rax create --size 50

# Or import an existing .rax vault (backing up any existing vault first)
rax import ~/Downloads/vault.rax --backup

# Expand your vault in-place by an additional 25 MB
rax grow 25

# Import a file from the host OS and remove the plaintext original
rax put ~/Documents/recovery_codes.txt --remove

# Launch the interactive rax shell
rax
```

### 2. Interactive Shell (`rax>`)
Once authenticated, `rax` drops you into an isolated shell with command history, context-aware tab completion, live syntax highlighting, and inline parameter hints:

```text
Info: Entering interactive shell. Type 'help' for commands, or 'exit' to quit.
rax> ls -d
Name                           | Size       | Created
-------------------------------+------------+---------------------
recovery_codes.txt             | 1.24 KB    | 2026-11-26 18:10:42
-------------------------------+------------+---------------------
Vault capacity: 1.00 MB / 75.00 MB used [1.34%]

rax> cat recovery_codes.txt
1. 8f9a-22b1-90c4
2. 7d1e-44a9-11b2

rax> edit recovery_codes.txt
✔ File 'recovery_codes.txt' successfully edited.

rax> get --file recovery_codes.txt --dest /tmp/exported_codes.txt
✔ 'recovery_codes.txt' successfully extracted.

rax> rm recovery_codes.txt
✔ File 'recovery_codes.txt' permanently removed from vault.

rax> exit
Info: Locking vault...
```

<details>
<summary><strong>📖 Essential Command Reference (Click to expand)</strong></summary>

### Host CLI Commands

| Command | Description |
| :--- | :--- |
| `rax` | Authenticate and open the interactive `rax>` shell |
| `rax create --size <MB>` | Create a new encrypted vault pre-filled with CSPRNG noise (`-s`) |
| `rax import <PATH> [--backup \| --force]` | Import an existing `.rax` vault into the default location. Validates `RAX1` magic bytes; refuses to overwrite unless `--backup` (saves existing vault as `.bak_<timestamp>`) or `--force` is specified |
| `rax grow <MB>` | Dynamically expand the vault container in-place by appending cryptographic noise |
| `rax put <PATH> [--remove]` | Encrypt and store a host file directly from the terminal (`-r` removes the plaintext original) |

### Interactive Shell (`rax>`) Commands

| Command | Description |
| :--- | :--- |
| `ls [-d \| --detail]` | List stored files; `-d` displays human-readable sizes, creation timestamps, and a vault capacity footer |
| `put <PATH> [-r \| --remove]` | Import a file from the host OS into the vault (supports host path tab completion) |
| `get -f <FILE> -d <DEST>` | Decrypt and extract a file from the vault to the host OS (`--file`, `--dest`) |
| `cat <FILE>` | Decrypt a UTF-8 text file directly into RAM and print it |
| `edit <FILE>` | Edit a text file in `/dev/shm` RAM disk using `$VISUAL`, `$EDITOR`, or `nano` |
| `rm <FILE>` | Permanently remove a file and deallocate its physical blocks |
| `grow <MB>` | Expand the active vault container in-place |

> Run `rax --help` on the host or `help` inside `rax>` to see all available commands and flags.

</details>

---

<a name="storage-architecture"></a>
## 🗄️ Storage Architecture

A `.rax` container is divided into a reserved **1 MB System Lobby** (Blocks `0..255`) and a **4096-Byte Data Grid** (Block `256+`):

```text
+-----------------------------------------------------------------------+
|                       1 MB SYSTEM LOBBY (Blocks 0..255)               |
|  [Magic: "RAX1" (4B)] [Master Salt (32B)] [Auth Chunk] [InodeTable]   |
+-----------------------------------------------------------------------+
|                       ENCRYPTED DATA GRID (Block 256..N)              |
|  Block 256: [12B Nonce | 4068B Encrypted Payload | 16B AES-GCM MAC]   |
|  Block 257: [12B Nonce | 4068B Encrypted Payload | 16B AES-GCM MAC]   |
|  ...                                                                  |
+-----------------------------------------------------------------------+
```

Full module documentation is automatically built and hosted via **GitHub Pages**:  
👉 **[Read the `rax` Rustdoc API Reference](https://Raiza04.github.io/rax/)**

---

<a name="limitations"></a>
## ⚠️ Current Limitations (v1.2.0)

In the spirit of transparent security engineering, please note the architectural constraints of the current `v1.2.0` release:

1. **Linux-Only In-Memory Editing (`/dev/shm`):** The `edit` command relies on `/dev/shm` (Shared Memory RAM disk) to pass decrypted files to external terminal editors (`vim`, `nano`) without touching the SSD. Consequently, `edit` is currently only supported on Linux. (A built-in cross-platform TUI editor is planned for `v2.0`).
2. **Flat File Structure:** `v1.2.0` uses a flat `InodeTable` without hierarchical subdirectories. Importing two files with the exact same filename is rejected to prevent collisions. Hierarchical directories (`mkdir`, `cd`, `pwd`) are scheduled for `v2.0`.
3. **1 MB Metadata Lobby Ceiling:** While `grow` enables dynamic in-place container expansion, the serialized `InodeTable` (file metadata + block allocation bitmap) must fit inside the reserved 1 MB system lobby; `grow` enforces this limit before touching disk. Fixed-block spanned headers are planned for `v2.0`.
4. **In-Memory File Streaming:** Commands like `cat` and `edit` decrypt the target file completely into a contiguous `Vec<u8>` buffer in RAM. While ideal for credentials, notes, source code, and documents, `v1.2.0` is not intended for multi-gigabyte video files.
5. **No Swap Locking (`mlock`) Yet:** While `zeroize` guarantees that buffers are wiped before deallocation, the OS kernel could theoretically swap active memory pages if system RAM is exhausted. OS-level memory locking (`mlock` / `VirtualLock`) is on the `v2.0` roadmap.

---

## 🤝 Development Story & Collaboration

`rax` was started as a deep-dive systems programming project to understand low-level file system internals, block allocation, and applied cryptography in Rust from the ground up.

* **Phase 1 — Solo Core Engineering:** The entire foundational architecture—including the binary container layout, the 4096-byte block allocation engine (`InodeTable`), the Argon2id/AES-256-GCM cryptographic pipeline, and the core vault I/O—was designed and written from scratch by the author to master Rust's ownership model and security primitives first-hand.
* **Phase 2 — AI-Assisted Pair Programming & Review:** Once the functional MVP backend stood, an AI assistant was integrated into the workflow in a reversed pair-programming role: the AI assisted with shell ergonomics, `zeroize` edge-case auditing, `/dev/shm` integration, comprehensive unit/negative testing, and CI/CD automation, while the author acted as the lead **System Architect and Code Reviewer**, scrutinizing every line for safety, performance, and architectural consistency.

---

## 🗺️ Roadmap Summary

* ✅ **v1.2.0 (UX & Vault Management — Released):** Context-aware tab completion, live syntax highlighting & inline hints in `rustyline`, dynamic in-place container resizing (`grow`), vault importing (`import`), detailed `ls -d` view, semantic ANSI output, and `indicatif` progress bars.
* 🚀 **v2.0.0 (Core VFS Overhaul — Planned):** Plausible Deniability (**Hidden Vaults** via dual-password lobby split), fixed-block headers, hierarchical directory trees (`mkdir`, `cd`, `pwd`, `mv`), `mlock` swap protection, and a built-in cross-platform TUI editor.

See [TODO.md](TODO.md) for the full technical backlog.

---

## 📄 License

Distributed under the MIT License. See `LICENSE` for more information.
