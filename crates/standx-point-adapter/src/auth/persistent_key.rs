/*
[INPUT]:  Wallet address and key storage directory
[OUTPUT]: Persistent Ed25519 signer instances
[POS]:    Auth layer - persistent storage for session-signing keys
[UPDATE]: When key storage format or file naming conventions change
*/

use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use crate::auth::Ed25519Signer;

/// Manages persistence of Ed25519 session keys
#[derive(Debug, Clone)]
pub struct PersistentKeyManager {
    key_dir: PathBuf,
}

impl PersistentKeyManager {
    /// Create a new key manager with the given storage directory
    pub fn new(key_dir: impl AsRef<Path>) -> Self {
        Self {
            key_dir: key_dir.as_ref().to_path_buf(),
        }
    }

    /// Get an existing signer or create a new one if it doesn't exist
    pub fn get_or_create_signer(&self, wallet_address: &str) -> io::Result<Ed25519Signer> {
        if let Some(signer) = self.load_signer(wallet_address) {
            Ok(signer)
        } else {
            let signer = Ed25519Signer::generate();
            self.save_signer(wallet_address, &signer)?;
            Ok(signer)
        }
    }

    /// Load a signer from disk for the given wallet address
    pub fn load_signer(&self, wallet_address: &str) -> Option<Ed25519Signer> {
        let path = self.key_file_path(wallet_address);
        let content = fs::read_to_string(path).ok()?;
        let bytes = STANDARD.decode(content.trim()).ok()?;

        if bytes.len() != 32 {
            return None;
        }

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&bytes);
        Some(Ed25519Signer::from_secret_key(&key_bytes))
    }

    /// Save a signer to disk for the given wallet address
    pub fn save_signer(&self, wallet_address: &str, signer: &Ed25519Signer) -> io::Result<()> {
        if !self.key_dir.exists() {
            fs::create_dir_all(&self.key_dir)?;
        }

        let path = self.key_file_path(wallet_address);
        let secret_bytes = signer.secret_key_bytes();
        let encoded = STANDARD.encode(secret_bytes);

        fs::write(&path, encoded)?;

        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&path, perms)?;

        Ok(())
    }

    /// List all wallet addresses that have stored keys
    pub fn list_stored_accounts(&self) -> Vec<String> {
        let mut accounts = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.key_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with("_ed25519.key") {
                        let address = &name[..name.len() - "_ed25519.key".len()];
                        accounts.push(address.to_string());
                    }
                }
            }
        }
        accounts
    }

    /// Get the expected file path for a wallet's key
    pub fn key_file_path(&self, wallet_address: &str) -> PathBuf {
        self.key_dir.join(format!("{}_ed25519.key", wallet_address))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use uuid::Uuid;

    fn temp_dir() -> PathBuf {
        let mut path = env::temp_dir();
        path.push(format!("standx-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn test_persistent_key_lifecycle() {
        let dir = temp_dir();
        let manager = PersistentKeyManager::new(&dir);
        let wallet = "0x1234abcd";

        let signer1 = manager.get_or_create_signer(wallet).unwrap();
        let pub_key1 = signer1.public_key_base58();

        let signer2 = manager
            .load_signer(wallet)
            .expect("Should load existing key");
        assert_eq!(signer2.public_key_base58(), pub_key1);

        let signer3 = manager.get_or_create_signer(wallet).unwrap();
        assert_eq!(signer3.public_key_base58(), pub_key1);

        let accounts = manager.list_stored_accounts();
        assert_eq!(accounts, vec![wallet]);

        let path = manager.key_file_path(wallet);
        let metadata = fs::metadata(path).unwrap();
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn test_multi_account_isolation() {
        let dir = temp_dir();
        let manager = PersistentKeyManager::new(&dir);

        let wallet1 = "0x1111";
        let wallet2 = "0x2222";

        let s1 = manager.get_or_create_signer(wallet1).unwrap();
        let s2 = manager.get_or_create_signer(wallet2).unwrap();

        assert_ne!(s1.public_key_base58(), s2.public_key_base58());

        let accounts = manager.list_stored_accounts();
        assert!(accounts.contains(&wallet1.to_string()));
        assert!(accounts.contains(&wallet2.to_string()));
        assert_eq!(accounts.len(), 2);

        fs::remove_dir_all(dir).unwrap();
    }
}
