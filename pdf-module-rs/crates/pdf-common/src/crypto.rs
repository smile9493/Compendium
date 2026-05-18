//! Cryptographic utilities: password hashing and authenticated encryption.
//!
//! Provides safe defaults for:
//! - Password hashing via argon2id (OWASP-recommended parameters)
//! - AES-256-GCM encryption/decryption via `aes-gcm` + `ring`
//! - Secure random byte generation
//!
//! # Feature flag
//!
//! Enable with `features = ["crypto"]` in `Cargo.toml`.
//!
//! # Usage
//!
//! ```ignore
//! use pdf_common::crypto::{hash_password, verify_password, encrypt, decrypt};
//!
//! let hash = hash_password("my-password")?;
//! assert!(verify_password("my-password", &hash)?);
//! ```

use aes_gcm::aead::{Aead, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key, KeyInit, Nonce};
use argon2::{
    password_hash::{rand_core, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, ParamsBuilder,
};
use ring::rand::{SecureRandom, SystemRandom};

/// Hash a password using argon2id with OWASP-recommended parameters.
///
/// # Recommended parameters
///
/// - Algorithm: argon2id (hybrid, resistant to both side-channel and GPU attacks)
/// - Memory: 19 MiB (OWASP minimum for argon2id)
/// - Iterations: 2 (combined with high memory for balanced security)
/// - Parallelism: 1 (prevents parallelism-based speedup)
///
/// # Example
///
/// ```ignore
/// let hash = hash_password("correct-horse-battery-staple")?;
/// ```
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let params = ParamsBuilder::new()
        .m_cost(19456)
        .t_cost(2)
        .p_cost(1)
        .output_len(32)
        .build()
        .map_err(|e| argon2::password_hash::Error::ParamNameInvalid(e.to_string()))?;

    let salt = SaltString::generate(&mut rand_core::OsRng);
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
}

/// Verify a password against an argon2id hash.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, argon2::password_hash::Error> {
    let parsed_hash = PasswordHash::new(hash)?;
    let argon2 = Argon2::default();
    Ok(argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

/// Generate `count` cryptographically secure random bytes.
pub fn secure_random_bytes(count: usize) -> Vec<u8> {
    let rng = SystemRandom::new();
    let mut bytes = vec![0u8; count];
    rng.fill(&mut bytes).expect("SystemRandom::fill failed");
    bytes
}

/// Generate a secure random hex string of `byte_len` bytes.
pub fn secure_random_hex(byte_len: usize) -> String {
    hex::encode(secure_random_bytes(byte_len))
}

/// AES-256-GCM encryption.
///
/// Returns `(nonce, ciphertext)` where nonce is 12 bytes (96 bits).
/// The nonce must be stored alongside the ciphertext for decryption.
pub fn aes_encrypt(plaintext: &[u8], key: &[u8; 32]) -> Result<(Vec<u8>, Vec<u8>), aes_gcm::Error> {
    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher.encrypt(&nonce, plaintext)?;
    Ok((nonce.to_vec(), ciphertext))
}

/// AES-256-GCM decryption.
pub fn aes_decrypt(
    nonce: &[u8],
    ciphertext: &[u8],
    key: &[u8; 32],
) -> Result<Vec<u8>, aes_gcm::Error> {
    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce);

    cipher.decrypt(nonce, ciphertext)
}

/// Encrypt a UTF-8 string and return base64-encoded `nonce:ciphertext`.
pub fn encrypt_string(plaintext: &str, key: &[u8; 32]) -> Result<String, aes_gcm::Error> {
    let (nonce, ciphertext) = aes_encrypt(plaintext.as_bytes(), key)?;
    let combined = [nonce.as_slice(), ciphertext.as_slice()].concat();
    Ok(base64_encode(&combined))
}

/// Decrypt a base64-encoded `nonce:ciphertext` string back to UTF-8.
pub fn decrypt_string(encoded: &str, key: &[u8; 32]) -> Result<String, aes_gcm::Error> {
    let combined = base64_decode(encoded);
    if combined.len() < 12 {
        return Err(aes_gcm::Error);
    }
    let (nonce, ciphertext) = combined.split_at(12);
    let plaintext = aes_decrypt(nonce, ciphertext, key)?;
    String::from_utf8(plaintext).map_err(|_| aes_gcm::Error)
}

fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

fn base64_decode(encoded: &str) -> Vec<u8> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_password() {
        let hash = hash_password("test-password-123").unwrap();
        assert!(verify_password("test-password-123", &hash).unwrap());
        assert!(!verify_password("wrong-password", &hash).unwrap());
    }

    #[test]
    fn hash_is_deterministically_different() {
        let h1 = hash_password("same-password").unwrap();
        let h2 = hash_password("same-password").unwrap();
        assert_ne!(h1, h2);
        assert!(verify_password("same-password", &h1).unwrap());
        assert!(verify_password("same-password", &h2).unwrap());
    }

    #[test]
    fn secure_random_hex_is_unique() {
        let r1 = secure_random_hex(32);
        let r2 = secure_random_hex(32);
        assert_ne!(r1, r2);
        assert_eq!(r1.len(), 64);
    }

    #[test]
    fn encrypt_and_decrypt_roundtrip() {
        let key: [u8; 32] = secure_random_bytes(32).try_into().unwrap();
        let plaintext = b"Hello, AES-256-GCM!";
        let (nonce, ciphertext) = aes_encrypt(plaintext, &key).unwrap();
        let decrypted = aes_decrypt(&nonce, &ciphertext, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn encrypt_string_roundtrip() {
        let key: [u8; 32] = secure_random_bytes(32).try_into().unwrap();
        let original = "Secret message: 你好世界";
        let encoded = encrypt_string(original, &key).unwrap();
        let decoded = decrypt_string(&encoded, &key).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn wrong_key_fails_decryption() {
        let key1: [u8; 32] = [1u8; 32];
        let key2: [u8; 32] = [2u8; 32];
        let (nonce, ciphertext) = aes_encrypt(b"test", &key1).unwrap();
        let result = aes_decrypt(&nonce, &ciphertext, &key2);
        assert!(result.is_err());
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let key: [u8; 32] = [3u8; 32];
        let (nonce, mut ciphertext) = aes_encrypt(b"test", &key).unwrap();
        ciphertext[0] ^= 0xFF;
        let result = aes_decrypt(&nonce, &ciphertext, &key);
        assert!(result.is_err());
    }
}