<div align="center">
<h1>🛡️ rax </h1>
  <p><strong>A secure, user-space Encrypted Virtual File System (VFS) written in Rust.</strong></p>

  <!-- Badges -->
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Made%20with-Rust-orange?style=for-the-badge&logo=rust" alt="Made with Rust"></a>
  <a href="https://github.com/rust-lang/rust"><img src="https://img.shields.io/badge/Rust-1.99+-blue?style=for-the-badge&logo=rust" alt="Rust Version"></a>
  <a href="#"><img src="https://img.shields.io/badge/Crypto-AES--256--GCM-success?style=for-the-badge&logo=expensify" alt="Crypto"></a>
  <a href="#"><img src="https://img.shields.io/badge/Hashing-Argon2-success?style=for-the-badge&logo=hashnode" alt="Hashing"></a>
  <a href="#"><img src="https://img.shields.io/badge/Platform-Linux%20%7C%20macOS-lightgrey?style=for-the-badge&logo=linux" alt="Platform"></a>
</div>

<br>

`rax` is a highly secure, lightweight virtual file system that stores your sensitive files inside a single, encrypted vault file. It comes with its own interactive REPL shell, allowing you to manage, view, and edit your encrypted files seamlessly without leaving the terminal.

---

## 🚀 Why `rax`? (vs. VeraCrypt)

You might be wondering: *"Why should I use `rax` when tools like VeraCrypt exist?"*

While VeraCrypt is a fantastic tool for full-disk or partition encryption, `rax` is built for a different workflow, solving specific pain points:

*   **No Root/Sudo Required:** VeraCrypt mounts volumes as actual block devices, which strictly requires administrator privileges. `rax` operates **100% in user-space**. You can run it on any machine where you have standard user access.
*   **Built-in REPL Shell:** Instead of mounting a drive and using your OS file explorer, `rax` isolates your workflow. You interact with your files through a custom, sandboxed terminal shell (`rax>`).
*   **Wear-Leveling Protection (The `edit` Command):** SSDs move data around (wear-leveling), making it incredibly hard to truly delete a decrypted file. When you edit a file in `rax`, it is extracted directly into a volatile RAM disk (`/dev/shm`), opened in your favorite `$EDITOR` (like `vim` or `nano`), and safely re-encrypted. The plaintext never touches your SSD.
*   **Lightweight & Portable:** A single binary. No kernel modules, no complex GUI. Just pure Rust CLI efficiency.

---

## 🔒 Security Architecture

`rax` takes data confidentiality and integrity very seriously:
- **AES-256-GCM (Galois/Counter Mode):** Every 4096-byte block is individually encrypted with a unique, randomly generated 12-byte Nonce. The GCM MAC tag guarantees tamper-proofing—if a single bit is altered maliciously, the decryption fails automatically.
- **Argon2 Key Derivation:** Master passwords are hashed using the memory-hard Argon2 algorithm alongside a 32-byte master salt, providing extreme resistance against brute-force and dictionary attacks.
- **Zeroize Memory Wipe:** Plaintext buffers and temporary files in the RAM disk are manually overwritten with zeros before being dropped, ensuring no residual data is left in the memory for other processes to snoop.
- **Gibberish Protection:** Strict UTF-8 validation prevents binary files from being rendered in terminal outputs (`cat`), avoiding terminal corruption.

---

## 📦 Installation

Ensure you have Rust and Cargo installed (`rustup`).

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/rax.git
   cd rax
   ```
2. Build and install the binary:
   ```bash
   cargo install --path .
   ```
3. Verify the installation:
   ```bash
   rax --help
   ```

---

## 💻 Getting Started

### 1. Create a Vault
Create a new encrypted vault (e.g., 50 MB in size). You will be prompted to set a master password.
```bash
rax create --size 50
```

### 2. Enter the Shell
Launch the interactive REPL shell. Enter your password when prompted.
```bash
rax
```

### 3. Shell Commands
Once inside the `rax>` prompt, you have access to a suite of commands:

| Command | Description | Example |
| :--- | :--- | :--- |
| `put <path>` | Imports a file from the host OS into the vault. Use `-r` to securely delete the original. | `put /home/user/secrets.txt -r` |
| `ls` | Lists all files currently in the vault with their sizes. | `ls` |
| `cat <file>` | Prints the decrypted content of a text file directly to the terminal. | `cat secrets.txt` |
| `edit <file>` | Securely opens the file in `$EDITOR` via a RAM disk, saving changes back. | `edit secrets.txt` |
| `get <file> <dest>`| Extracts a file from the vault to the host OS. | `get secrets.txt /tmp/out.txt` |
| `rm <file>` | Permanently removes a file and deallocates its blocks in the vault. | `rm secrets.txt` |
| `clear` | Clears the terminal screen. | `clear` |
| `exit` | Locks the vault safely and exits the shell. | `exit` |

---

## 🤝 The Story & Collaboration

This project was born out of a desire to deep-dive into Rust and understand the low-level mechanics of Virtual File Systems (VFS) and Cryptography. 

**Phase 1: Architecture & MVP (Human)**
The entire core architecture—the `InodeTable`, the 4096-byte block grid layout, the AES-GCM integration, and the foundational backend—was architected and written by me. The goal was to learn Rust's ownership model, traits, and binary serialization the hard way.

**Phase 2: Refinement & Pair-Programming (Human + AI)**
Once the Minimum Viable Product (MVP) was running, the development dynamic shifted. I brought an AI (LLM) on board as a "Pair Programmer", while I transitioned into the role of **Lead Architect and Code Reviewer**. 
Under my strict code reviews, the AI was tasked with writing professional Doxygen-style documentation (`rustdoc`), generating rigorous edge-case unit tests, and implementing the volatile RAM-disk (`/dev/shm`) `edit` feature. 

This hybrid approach not only accelerated the polishing phase but also proved to be an incredibly effective and modern way to build robust, production-ready software.

---

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
