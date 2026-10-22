# local-password-vault

An encrypted local password manager with a master password, built in Rust.

This project is a hands-on way to practice:

- **Key derivation** – turning the master password into an encryption key with [Argon2](https://crates.io/crates/argon2).
- **Authenticated encryption** – protecting the vault with [`aes-gcm`](https://crates.io/crates/aes-gcm) or [`chacha20poly1305`](https://crates.io/crates/chacha20poly1305).
- **Safe secret handling in memory** – wiping sensitive data after use with [`zeroize`](https://crates.io/crates/zeroize).

## How it works

1. On first run, the user picks a master password. A random salt is generated.
2. Argon2 derives a 256-bit key from the master password and salt.
3. Vault entries are serialized and encrypted with AES-256-GCM (or ChaCha20-Poly1305) using a fresh random nonce.
4. The salt, Argon2 parameters, nonce and ciphertext are stored in a single vault file. The master password and derived key are never written to disk.
5. On unlock, the key is re-derived; a failed authentication tag means a wrong password or a tampered file.
6. Secrets are held in `Zeroizing`/`Secret` wrappers so they are wiped from memory when dropped.

## Project structure

```
local-password-vault/
├── Cargo.toml
├── README.md
├── .gitignore
├── src/
│   ├── main.rs        # Entry point: parses CLI and dispatches commands
│   ├── cli.rs         # Command-line definitions (init, add, get, list, remove, change-password)
│   ├── crypto/
│   │   ├── mod.rs
│   │   ├── kdf.rs     # Argon2 key derivation (salt, parameters)
│   │   └── cipher.rs  # AES-GCM / ChaCha20-Poly1305 encrypt & decrypt
│   ├── vault/
│   │   ├── mod.rs
│   │   ├── entry.rs   # Entry model (name, username, password, notes)
│   │   ├── store.rs   # Load/save the encrypted vault file
│   │   └── format.rs  # On-disk format (version, salt, KDF params, nonce, ciphertext)
│   ├── secret.rs      # Zeroize-backed wrappers for passwords and keys
│   └── error.rs       # Error types
└── tests/
    └── vault_roundtrip.rs  # Integration tests (encrypt/decrypt, wrong password, tampering)
```

## Suggested dependencies

```toml
[dependencies]
argon2 = "0.5"
aes-gcm = "0.10"          # or chacha20poly1305 = "0.10"
zeroize = { version = "1", features = ["derive"] }
rand = "0.8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive"] }
rpassword = "7"
thiserror = "1"
```

## Getting started

```sh
cargo new --bin local-password-vault   # or use this repository directly
cargo build --release
cargo run -- init
cargo run -- add example.com
cargo run -- get example.com
cargo test
```

## Security notes

- Use a unique random nonce for every encryption; never reuse a nonce with the same key.
- Read the master password without echo (`rpassword`) and zeroize it after key derivation.
- Choose Argon2id with memory-hard parameters and store them in the vault header.
- Keep the vault file permissions restricted (`0600`) and write it atomically.
- This is a learning project and has not been audited; do not rely on it for critical secrets.
