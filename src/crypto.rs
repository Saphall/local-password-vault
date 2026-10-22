use std::fmt;

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, aead::Aead};
use zeroize::Zeroizing;

pub const SALT_LEN: usize = 16;
pub const NONCE_LEN: usize = 24;
pub const KEY_LEN: usize = 32;

#[derive(Debug)]
pub enum CryptoError {
    Kdf(String),
    Encrypt,
    /// Wrong key or tampered data; AEAD cannot tell these apart.
    Decrypt,
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CryptoError::Kdf(e) => write!(f, "key derivation failed: {e}"),
            CryptoError::Encrypt => write!(f, "encryption failed"),
            CryptoError::Decrypt => write!(f, "decryption failed"),
        }
    }
}

impl std::error::Error for CryptoError {}

pub fn random_bytes<const N: usize>() -> [u8; N] {
    let mut buf = [0u8; N];
    rand::fill(&mut buf);
    buf
}

/// Derives a 256-bit key from the master password with Argon2id.
pub fn derive_key(master: &str, salt: &[u8]) -> Result<Zeroizing<[u8; KEY_LEN]>, CryptoError> {
    let params = Params::new(64 * 1024, 3, 1, Some(KEY_LEN)).map_err(|e| CryptoError::Kdf(e.to_string()))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    argon2
        .hash_password_into(master.as_bytes(), salt, key.as_mut())
        .map_err(|e| CryptoError::Kdf(e.to_string()))?;
    Ok(key)
}

/// Encrypts with a fresh random nonce, which must be stored next to the ciphertext.
pub fn encrypt(key: &[u8; KEY_LEN], plaintext: &[u8]) -> Result<([u8; NONCE_LEN], Vec<u8>), CryptoError> {
    let cipher = XChaCha20Poly1305::new_from_slice(key).map_err(|_| CryptoError::Encrypt)?;
    let nonce = random_bytes::<NONCE_LEN>();
    let ciphertext = cipher
        .encrypt(&nonce.into(), plaintext)
        .map_err(|_| CryptoError::Encrypt)?;
    Ok((nonce, ciphertext))
}

pub fn decrypt(key: &[u8; KEY_LEN], nonce: &[u8; NONCE_LEN], ciphertext: &[u8]) -> Result<Zeroizing<Vec<u8>>, CryptoError> {
    let cipher = XChaCha20Poly1305::new_from_slice(key).map_err(|_| CryptoError::Decrypt)?;
    cipher
        .decrypt(&(*nonce).into(), ciphertext)
        .map(Zeroizing::new)
        .map_err(|_| CryptoError::Decrypt)
}
