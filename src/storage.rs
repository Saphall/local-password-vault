use std::{
    fmt, fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::{
    crypto::{self, CryptoError, KEY_LEN, NONCE_LEN, SALT_LEN},
    models::Entry,
};

const FORMAT_VERSION: u32 = 1;

#[derive(Debug)]
pub enum StorageError {
    Io(io::Error),
    AlreadyExists,
    Corrupt,
    WrongPassword,
    Crypto(CryptoError),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::Io(e) => write!(f, "file error: {e}"),
            StorageError::AlreadyExists => write!(f, "a vault already exists at this location"),
            StorageError::Corrupt => write!(f, "vault file is corrupt"),
            StorageError::WrongPassword => write!(f, "wrong master password"),
            StorageError::Crypto(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<io::Error> for StorageError {
    fn from(e: io::Error) -> Self {
        StorageError::Io(e)
    }
}

impl From<CryptoError> for StorageError {
    fn from(e: CryptoError) -> Self {
        StorageError::Crypto(e)
    }
}

/// On-disk format. The master password and key are never stored.
#[derive(Serialize, Deserialize)]
struct VaultFile {
    version: u32,
    salt: String,
    nonce: String,
    ciphertext: String,
}

pub struct Vault {
    path: PathBuf,
    salt: [u8; SALT_LEN],
    key: Zeroizing<[u8; KEY_LEN]>,
    pub entries: Vec<Entry>,
}

impl Vault {
    pub fn default_path() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("passlock")
            .join("vault.passlock")
    }

    pub fn exists(path: &Path) -> bool {
        path.exists()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn create(path: PathBuf, master: &str) -> Result<Self, StorageError> {
        if path.exists() {
            return Err(StorageError::AlreadyExists);
        }
        let salt = crypto::random_bytes::<SALT_LEN>();
        let vault = Vault {
            key: crypto::derive_key(master, &salt)?,
            path,
            salt,
            entries: Vec::new(),
        };
        vault.save()?;
        Ok(vault)
    }

    pub fn unlock(path: PathBuf, master: &str) -> Result<Self, StorageError> {
        let file: VaultFile = serde_json::from_str(&fs::read_to_string(&path)?).map_err(|_| StorageError::Corrupt)?;
        if file.version != FORMAT_VERSION {
            return Err(StorageError::Corrupt);
        }
        let salt: [u8; SALT_LEN] = unhex(&file.salt)?.try_into().map_err(|_| StorageError::Corrupt)?;
        let nonce: [u8; NONCE_LEN] = unhex(&file.nonce)?.try_into().map_err(|_| StorageError::Corrupt)?;
        let ciphertext = unhex(&file.ciphertext)?;

        let key = crypto::derive_key(master, &salt)?;
        let plain = crypto::decrypt(&key, &nonce, &ciphertext).map_err(|_| StorageError::WrongPassword)?;
        let entries = serde_json::from_slice(&plain).map_err(|_| StorageError::Corrupt)?;
        Ok(Vault { path, salt, key, entries })
    }

    /// Re-encrypts with a fresh nonce and replaces the file atomically.
    pub fn save(&self) -> Result<(), StorageError> {
        let plain = Zeroizing::new(serde_json::to_vec(&self.entries).map_err(|_| StorageError::Corrupt)?);
        let (nonce, ciphertext) = crypto::encrypt(&self.key, &plain)?;
        let file = VaultFile {
            version: FORMAT_VERSION,
            salt: hex(&self.salt),
            nonce: hex(&nonce),
            ciphertext: hex(&ciphertext),
        };
        let json = serde_json::to_vec_pretty(&file).map_err(|_| StorageError::Corrupt)?;

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = self.path.with_extension("tmp");
        write_private(&tmp, &json)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

fn write_private(path: &Path, data: &[u8]) -> io::Result<()> {
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(path)?;
    f.write_all(data)?;
    f.sync_all()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Result<Vec<u8>, StorageError> {
    if !s.is_ascii() || s.len() % 2 != 0 {
        return Err(StorageError::Corrupt);
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| StorageError::Corrupt))
        .collect()
}
