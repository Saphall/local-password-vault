use passlock::crypto::{self, NONCE_LEN, SALT_LEN};

#[test]
fn encrypt_then_decrypt_roundtrips() {
    let key = crypto::derive_key("master", &crypto::random_bytes::<SALT_LEN>()).unwrap();
    let (nonce, ct) = crypto::encrypt(&key, b"hello vault").unwrap();
    assert_ne!(ct.as_slice(), b"hello vault");
    assert_eq!(crypto::decrypt(&key, &nonce, &ct).unwrap().as_slice(), b"hello vault");
}

#[test]
fn same_password_and_salt_give_same_key() {
    let salt = crypto::random_bytes::<SALT_LEN>();
    let a = crypto::derive_key("master", &salt).unwrap();
    let b = crypto::derive_key("master", &salt).unwrap();
    let c = crypto::derive_key("other", &salt).unwrap();
    assert_eq!(*a, *b);
    assert_ne!(*a, *c);
}

#[test]
fn wrong_key_fails_to_decrypt() {
    let salt = crypto::random_bytes::<SALT_LEN>();
    let key = crypto::derive_key("one", &salt).unwrap();
    let other = crypto::derive_key("two", &salt).unwrap();
    let (nonce, ct) = crypto::encrypt(&key, b"data").unwrap();
    assert!(crypto::decrypt(&other, &nonce, &ct).is_err());
}

#[test]
fn nonces_are_unique_per_encryption() {
    let key = crypto::derive_key("master", &[0u8; SALT_LEN]).unwrap();
    let (n1, _) = crypto::encrypt(&key, b"x").unwrap();
    let (n2, _) = crypto::encrypt(&key, b"x").unwrap();
    assert_ne!(n1, n2);
    assert_eq!(n1.len(), NONCE_LEN);
}
