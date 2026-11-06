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

[Features](#-key-security-features) •
[Why rax?](#-why-rax-instead-of-veracrypt) •
[Installation](#-installation) •
[Usage](#-usage) •
[Architecture](#-storage-architecture) •
[Limitations](#️-current-limitations-v010) •
[API Docs](https://Raiza04.github.io/rax/)

</div>

---

## 🛡️ Overview

`rax` is a lightweight, portable **Encrypted Virtual File System (VFS)** designed to protect sensitive text files, credentials, and documents against SSD wear-leveling leaks and volatile memory remnants.

Unlike traditional disk encryption tools that mount an entire filesystem into the host OS kernel—exposing decrypted files to background indexers, swap files, and OS telemetry—`rax` operates strictly in **isolated user-space**. You interact with your encrypted container through an integrated, memory-safe Read-Eval-Print Loop (REPL) shell (`rax>`).

---

## ✨ Key Security Features

* **🔒 Authenticated Block Cryptography:** Uses **AES-256-GCM** mapped to a fixed `4096`-byte physical block grid (`12`-byte CSPRNG Nonce + `4068`-byte payload + `16`-byte Poly1305 MAC tag). Every block is cryptographically authenticated to detect bit-rot or tampering immediately.
* **🧠 Memory-Hard Key Derivation:** Master keys are derived on the fly via **Argon2id** paired with a 32-byte per-vault cryptographic salt, defending against GPU-accelerated brute-force attacks.
* **🧹 Volatile RAM Wiping (`zeroize`):** All plaintext file buffers, derived key slices, and password strings are explicitly overwritten with zeros in RAM using volatile memory operations before going out of scope.
* **⚡ Zero-SSD-Trace Editing (`/dev/shm`):** When editing a file via `edit`, `rax` extracts the decrypted stream exclusively into Linux Shared Memory (`/dev/shm` RAM disk), launches your `$VISUAL` / `$EDITOR` (or `nano`), re-encrypts the updated file on exit, and zeros out the volatile buffer—never touching physical NAND flash.
* **🎲 Cryptographic Noise Initialization:** Newly created `.rax` containers are pre-filled with CSPRNG noise from byte zero to mask actual file allocation boundaries.

---

## ⚖️ Why `rax` Instead of VeraCrypt?

| Feature / Threat Vector | `rax` (User-Space VFS) | VeraCrypt / LUKS (Kernel Mount) |
| :--- | :--- | :--- |
| **Root / `sudo` Privileges** | **Not required** (runs in user-space) | **Required** to mount virtual block devices |
| **OS Indexing & Telemetry** | **Immune** (OS cannot see inside the open vault) | **Exposed** (Spotlight, Windows Search, & thumbs index mounted drives) |
| **SSD Wear-Leveling Leaks** | **Prevented** (`cat` and `edit` stay strictly in RAM) | **Risk via Temp Files** (editors often write `.swp` files to host disk) |
| **Explicit RAM Zeroization** | **Enforced** after every single command | Keys stay mounted in kernel memory until dismount |
| **Workflow** | Instant CLI commands & dedicated `rax>` shell | Heavy GUI or multi-step loop-device mounting |

---

## 📦 Installation

### 1. Quick Install (One-Liner Binary for Linux x86_64)
Downloads the latest precompiled release binary, verifies its SHA-256 checksum, and installs `rax` globally into `/usr/local/bin`:

```bash
curl -sSL [https://raw.githubusercontent.com/Raiza04/rax/master/install.sh](https://raw.githubusercontent.com/Raiza04/rax/master/install.sh) | bash
```

### 2. Install via Cargo (One-Liner from Source)
If you have the Rust toolchain installed, Cargo can compile and place the binary directly into `~/.cargo/bin/rax`:

```bash
cargo install --git [https://github.com/Raiza04/rax.git](https://github.com/Raiza04/rax.git)
```

### 3. Manual Build from Source
```bash
git clone [https://github.com/Raiza04/rax.git](https://github.com/Raiza04/rax.git)
cd rax
cargo build --release

# Install binary globally so you can run 'rax' from anywhere
sudo install -m 755 target/release/rax /usr/local/bin/rax
```

---

## 🚀 Usage

### 1. Host Terminal Commands (Bash / Zsh)
Initialize a new encrypted vault (stored at `~/.local/share/rax/vault.rax` by default) or import files directly from your host shell:

```bash
# Create a new 50 MB encrypted vault
rax create --size 50

# Import a file directly from the host OS and delete the plaintext original
rax put ~/Documents/recovery_codes.txt --remove

# Launch the interactive rax shell
rax
```

### 2. Interactive Shell (`rax>`)
Once authenticated, `rax` drops you into an isolated shell environment with command history:

```text
Welcome to the rax shell. Type 'exit' to quit.
rax> ls
Name                           | Size
-------------------------------+-----------
recovery_codes.txt             | 1 KB

rax> cat recovery_codes.txt
1. 8f9a-22b1-90c4
2. 7d1e-44a9-11b2

rax> edit recovery_codes.txt
File 'recovery_codes.txt' successfully updated.

rax> get --file recovery_codes.txt --dest /tmp/exported_codes.txt
'recovery_codes.txt' successfully extracted.

rax> rm recovery_codes.txt
File 'recovery_codes.txt' permanently removed from vault.

rax> clear
rax> exit
Locking vault...
```

---

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

## ⚠️ Current Limitations (v0.1.0)

In the spirit of transparent security engineering, please note the architectural constraints of the current `v0.1.0` MVP release:

1. **Linux-Only In-Memory Editing (`/dev/shm`):** The `edit` command relies on `/dev/shm` (Shared Memory RAM disk) to pass decrypted files to external terminal editors (`vim`, `nano`) without touching the SSD. Consequently, `edit` is currently only supported on Linux. (A built-in cross-platform TUI editor is planned for `v2.0`).
2. **Flat File Structure:** `v0.1.0` uses a flat `InodeTable` without hierarchical subdirectories. Importing two files with the exact same filename is rejected to prevent collisions. Hierarchical directories (`mkdir`, `cd`, `pwd`) are scheduled for `v2.0`.
3. **Static Container Size:** The total size of `vault.rax` is fixed upon creation (`rax create --size <MB>`). Dynamic in-place container expansion (`grow`) will be introduced in `v0.2.0`.
4. **In-Memory File Streaming:** Commands like `cat` and `edit` decrypt the target file completely into a contiguous `Vec<u8>` buffer in RAM. While ideal for credentials, notes, source code, and documents, `v0.1.0` is not intended for multi-gigabyte video files.
5. **No Swap Locking (`mlock`) Yet:** While `zeroize` guarantees that buffers are wiped before deallocation, the OS kernel could theoretically swap active memory pages if system RAM is exhausted. OS-level memory locking (`mlock` / `VirtualLock`) is on the `v2.0` roadmap.

---

## 🤝 Development Story & Collaboration

`rax` was started as a deep-dive systems programming project to understand low-level file system internals, block allocation, and applied cryptography in Rust from the ground up.

* **Phase 1 — Solo Core Engineering:** The entire foundational architecture—including the binary container layout, the 4096-byte block allocation engine (`InodeTable`), the Argon2id/AES-256-GCM cryptographic pipeline, and the core vault I/O—was designed and written from scratch by the author to master Rust's ownership model and security primitives first-hand.
* **Phase 2 — AI-Assisted Pair Programming & Review:** Once the functional MVP backend stood, an AI assistant was integrated into the workflow in a reversed pair-programming role: the AI assisted with shell ergonomics, `zeroize` edge-case auditing, `/dev/shm` integration, comprehensive unit/negative testing, and CI/CD automation, while the author acted as the lead **System Architect and Code Reviewer**, scrutinizing every line for safety, performance, and architectural consistency.

---

## 🗺️ Roadmap Summary

* **v1.2.0 (UX & Polish):** Tab-completion in `rustyline`, dynamic container resizing (`grow`), colorized output, and progress bars.
* **v2.0.0 (Core VFS Overhaul):** Plausible Deniability (**Hidden Vaults** via dual-password lobby split), fixed-block headers, hierarchical directory trees (`mkdir`, `cd`), and `mlock` swap protection.

---

## 📄 License

Distributed under the MIT License. See `LICENSE` for more information.
