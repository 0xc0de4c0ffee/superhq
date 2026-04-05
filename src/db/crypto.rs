use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, AeadCore, Nonce};
use anyhow::{bail, Context, Result};
use std::path::PathBuf;

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;

/// Path to the encryption key file.
fn key_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("superhq")
        .join("encryption.key")
}

/// Load the encryption key from disk, or generate and persist a new one.
pub fn load_or_create_key() -> Result<[u8; KEY_LEN]> {
    let path = key_path();

    if path.exists() {
        let data = std::fs::read(&path).context("reading encryption key")?;
        if data.len() != KEY_LEN {
            bail!(
                "encryption key file has wrong size: expected {KEY_LEN}, got {}",
                data.len()
            );
        }
        let mut key = [0u8; KEY_LEN];
        key.copy_from_slice(&data);
        return Ok(key);
    }

    // Generate a new key
    let mut key = [0u8; KEY_LEN];
    getrandom::getrandom(&mut key).map_err(|e| anyhow::anyhow!("generating encryption key: {e}"))?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, &key)?;

    // Set 0600 permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(key)
}

/// Generate an ephemeral key for testing (not persisted).
pub fn ephemeral_key() -> [u8; KEY_LEN] {
    let mut key = [0u8; KEY_LEN];
    getrandom::getrandom(&mut key).expect("getrandom failed");
    key
}

/// Encrypt plaintext with AES-256-GCM. Returns `[12-byte nonce | ciphertext+tag]`.
pub fn encrypt(plaintext: &[u8], key: &[u8; KEY_LEN]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key).context("creating cipher")?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| anyhow::anyhow!("encryption failed: {e}"))?;

    let mut blob = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    blob.extend_from_slice(&nonce);
    blob.extend_from_slice(&ciphertext);
    Ok(blob)
}

/// Decrypt a blob produced by `encrypt()`. Returns plaintext bytes.
pub fn decrypt(blob: &[u8], key: &[u8; KEY_LEN]) -> Result<Vec<u8>> {
    if blob.len() < NONCE_LEN {
        bail!("ciphertext too short");
    }
    let (nonce_bytes, ciphertext) = blob.split_at(NONCE_LEN);
    let nonce = Nonce::from_slice(nonce_bytes);
    let cipher = Aes256Gcm::new_from_slice(key).context("creating cipher")?;
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("decryption failed (wrong key or tampered data): {e}"))?;
    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let key = ephemeral_key();
        let plaintext = b"sk-ant-api03-secret-key-here";
        let blob = encrypt(plaintext, &key).unwrap();
        let decrypted = decrypt(&blob, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn wrong_key_fails() {
        let key1 = ephemeral_key();
        let key2 = ephemeral_key();
        let blob = encrypt(b"secret", &key1).unwrap();
        assert!(decrypt(&blob, &key2).is_err());
    }

    #[test]
    fn tampered_blob_fails() {
        let key = ephemeral_key();
        let mut blob = encrypt(b"secret", &key).unwrap();
        // Flip a byte in the ciphertext
        let last = blob.len() - 1;
        blob[last] ^= 0xff;
        assert!(decrypt(&blob, &key).is_err());
    }
}
