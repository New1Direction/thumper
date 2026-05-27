//! Cryptographic node keypair management for provenance and verifiable execution signatures.

use anyhow::{anyhow, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use std::fs;
use std::path::PathBuf;

fn key_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".api-anything").join("node_key.bin")
}

/// Load the persistent node Ed25519 keypair from disk, or generate a new one if missing.
pub fn init_node_key() -> Result<SigningKey> {
    let path = key_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }

    if path.exists() {
        if let Ok(bytes) = fs::read(&path) {
            if bytes.len() == 32 {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                return Ok(SigningKey::from_bytes(&arr));
            }
        }
    }

    // Generate a fresh cryptographically secure Ed25519 signing key
    let mut rng = rand::thread_rng();
    let signing_key = SigningKey::generate(&mut rng);
    fs::write(&path, signing_key.to_bytes())?;
    Ok(signing_key)
}

/// Sign a block of arbitrary data using the node's private key.
/// Returns the hexadecimal representation of the public key and signature.
pub fn sign_data(data: &[u8]) -> Result<(String, String)> {
    let key = init_node_key()?;
    let sig = key.sign(data);
    let pubkey = key.verifying_key();

    Ok((hex::encode(pubkey.to_bytes()), hex::encode(sig.to_bytes())))
}

/// Verify that a hexadecimal Ed25519 signature is valid for a given block of data and public key.
pub fn verify_data(pubkey_hex: &str, data: &[u8], sig_hex: &str) -> Result<bool> {
    let pubkey_bytes = hex::decode(pubkey_hex)?;
    let sig_bytes = hex::decode(sig_hex)?;

    let pubkey_arr: [u8; 32] = pubkey_bytes
        .try_into()
        .map_err(|_| anyhow!("Invalid pubkey length"))?;
    let sig_arr: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| anyhow!("Invalid signature length"))?;

    let pubkey = VerifyingKey::from_bytes(&pubkey_arr)?;
    let sig = Signature::from_bytes(&sig_arr);

    Ok(pubkey.verify(data, &sig).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_key_signing_and_verification() {
        let temp_dir = tempfile::tempdir().unwrap();
        let key_file = temp_dir.path().join("node_key.bin");

        // Override path during tests
        let signing_key = {
            let mut rng = rand::thread_rng();
            let key = SigningKey::generate(&mut rng);
            fs::write(&key_file, key.to_bytes()).unwrap();
            key
        };

        // Assert serialization and deserialization
        let loaded_bytes = fs::read(&key_file).unwrap();
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&loaded_bytes);
        let loaded_key = SigningKey::from_bytes(&arr);
        assert_eq!(signing_key.to_bytes(), loaded_key.to_bytes());

        // Assert signing and verification works
        let data = b"Thumper Verifiable DAG Execution Proof";
        let sig = signing_key.sign(data);
        let pubkey = signing_key.verifying_key();

        let pubkey_hex = hex::encode(pubkey.to_bytes());
        let sig_hex = hex::encode(sig.to_bytes());

        let is_valid = verify_data(&pubkey_hex, data, &sig_hex).unwrap();
        assert!(is_valid);

        let altered_data = b"Thumper Tampered Execution Proof";
        let is_invalid = verify_data(&pubkey_hex, altered_data, &sig_hex).unwrap();
        assert!(!is_invalid);
    }
}
