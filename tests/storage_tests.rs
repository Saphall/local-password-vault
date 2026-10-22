use std::{env, fs, path::PathBuf};

use passlock::{crypto, storage::{StorageError, Vault}};

fn temp_vault_path(tag: &str) -> PathBuf {
    let dir = env::temp_dir().join(format!("passlock-test-{tag}-{}", crypto::random_bytes::<4>().map(|b| format!("{b:02x}")).concat()));
    fs::create_dir_all(&dir).unwrap();
    dir.join("vault.passlock")
}

#[test]
fn entries_survive_lock_and_unlock() {
    let path = temp_vault_path("roundtrip");
    let mut vault = Vault::create(path.clone(), "correct horse").unwrap();
    vault.entries.push(passlock::models::Entry {
        id: 1,
        name: "github".into(),
        password: "s3cret!".into(),
        ..Default::default()
    });
    vault.save().unwrap();

    let reopened = Vault::unlock(path, "correct horse").unwrap();
    assert_eq!(reopened.entries.len(), 1);
    assert_eq!(reopened.entries[0].password, "s3cret!");
}

#[test]
fn wrong_master_password_is_rejected() {
    let path = temp_vault_path("wrong");
    Vault::create(path.clone(), "right password").unwrap();
    assert!(matches!(Vault::unlock(path, "wrong password"), Err(StorageError::WrongPassword)));
}

#[test]
fn file_contains_no_plaintext() {
    let path = temp_vault_path("plain");
    let mut vault = Vault::create(path.clone(), "master-pass").unwrap();
    vault.entries.push(passlock::models::Entry {
        id: 1,
        name: "uniquesitename".into(),
        password: "uniquepassword".into(),
        ..Default::default()
    });
    vault.save().unwrap();

    let raw = fs::read_to_string(path).unwrap();
    assert!(!raw.contains("uniquesitename") && !raw.contains("uniquepassword") && !raw.contains("master-pass"));
}

#[test]
fn create_refuses_to_overwrite() {
    let path = temp_vault_path("exists");
    Vault::create(path.clone(), "first password").unwrap();
    assert!(matches!(Vault::create(path, "second password"), Err(StorageError::AlreadyExists)));
}

#[test]
fn tampered_file_is_rejected() {
    let path = temp_vault_path("tamper");
    Vault::create(path.clone(), "master-pass").unwrap();
    let raw = fs::read_to_string(&path).unwrap();
    let mut json: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let ct = json["ciphertext"].as_str().unwrap().to_string();
    let flipped = if ct.starts_with('0') { format!("1{}", &ct[1..]) } else { format!("0{}", &ct[1..]) };
    json["ciphertext"] = flipped.into();
    fs::write(&path, json.to_string()).unwrap();
    assert!(Vault::unlock(path, "master-pass").is_err());
}
