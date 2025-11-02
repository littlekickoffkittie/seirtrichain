//! Wallet management for siertrichain
//!
//! Provides functionality for creating, loading, and managing wallets
//! that store keypairs and track triangle ownership.

use crate::crypto::KeyPair;
use crate::error::ChainError;
use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Wallet data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    /// Optional wallet name
    pub name: Option<String>,
    /// Wallet address (derived from public key)
    pub address: String,
    /// Secret key (hex-encoded)
    #[serde(rename = "secret_key")]
    pub secret_key_hex: String,
    /// Creation timestamp
    pub created: String,
}

impl Wallet {
    /// Create a new wallet with a generated keypair
    pub fn new(name: Option<String>) -> Result<Self, ChainError> {
        let keypair = KeyPair::generate()?;
        let address = keypair.address();
        let secret_key_hex = hex::encode(keypair.secret_key.secret_bytes());

        Ok(Wallet {
            name,
            address,
            secret_key_hex,
            created: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Load a wallet from a file
    pub fn load(path: &PathBuf) -> Result<Self, ChainError> {
        let contents = fs::read_to_string(path)
            .map_err(|e| ChainError::WalletError(format!("Failed to read wallet: {}", e)))?;

        let wallet: Wallet = serde_json::from_str(&contents)
            .map_err(|e| ChainError::WalletError(format!("Failed to parse wallet: {}", e)))?;

        Ok(wallet)
    }

    /// Save the wallet to a file
    pub fn save(&self, path: &PathBuf) -> Result<(), ChainError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| ChainError::WalletError(format!("Failed to serialize wallet: {}", e)))?;

        fs::write(path, json)
            .map_err(|e| ChainError::WalletError(format!("Failed to write wallet: {}", e)))?;

        Ok(())
    }

    /// Get the keypair from the wallet
    pub fn get_keypair(&self) -> Result<KeyPair, ChainError> {
        let secret_bytes = hex::decode(&self.secret_key_hex)
            .map_err(|e| ChainError::WalletError(format!("Failed to decode secret key: {}", e)))?;

        KeyPair::from_secret_bytes(&secret_bytes)
    }
}

/// Get the default wallet directory
pub fn get_wallet_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".siertrichain")
}

/// Get the default wallet file path
pub fn get_default_wallet_path() -> PathBuf {
    get_wallet_dir().join("wallet.json")
}

/// Get a named wallet file path
pub fn get_named_wallet_path(name: &str) -> PathBuf {
    get_wallet_dir().join(format!("wallet_{}.json", name))
}

/// Create the wallet directory if it doesn't exist
pub fn ensure_wallet_dir() -> Result<(), ChainError> {
    let wallet_dir = get_wallet_dir();
    fs::create_dir_all(&wallet_dir)
        .map_err(|e| ChainError::WalletError(format!("Failed to create wallet directory: {}", e)))?;
    Ok(())
}

/// Create a new wallet and save it to the default location
pub fn create_default_wallet() -> Result<Wallet, ChainError> {
    ensure_wallet_dir()?;

    let path = get_default_wallet_path();

    if path.exists() {
        return Err(ChainError::WalletError(
            "Wallet already exists at default location".to_string()
        ));
    }

    let wallet = Wallet::new(None)?;
    wallet.save(&path)?;

    Ok(wallet)
}

/// Create a named wallet
pub fn create_named_wallet(name: &str) -> Result<Wallet, ChainError> {
    ensure_wallet_dir()?;

    let path = get_named_wallet_path(name);

    if path.exists() {
        return Err(ChainError::WalletError(
            format!("Wallet '{}' already exists", name)
        ));
    }

    let wallet = Wallet::new(Some(name.to_string()))?;
    wallet.save(&path)?;

    Ok(wallet)
}

/// Load the default wallet
pub fn load_default_wallet() -> Result<Wallet, ChainError> {
    let path = get_default_wallet_path();

    if !path.exists() {
        return Err(ChainError::WalletError(
            "No wallet found. Run 'siertri-wallet new' first.".to_string()
        ));
    }

    Wallet::load(&path)
}

/// Load a named wallet
pub fn load_named_wallet(name: &str) -> Result<Wallet, ChainError> {
    let path = get_named_wallet_path(name);

    if !path.exists() {
        return Err(ChainError::WalletError(
            format!("Wallet '{}' not found", name)
        ));
    }

    Wallet::load(&path)
}

/// List all available wallets in the wallet directory
pub fn list_wallets() -> Result<Vec<String>, ChainError> {
    let wallet_dir = get_wallet_dir();

    if !wallet_dir.exists() {
        return Ok(Vec::new());
    }

    let mut wallets = Vec::new();

    let entries = fs::read_dir(&wallet_dir)
        .map_err(|e| ChainError::WalletError(format!("Failed to read wallet directory: {}", e)))?;

    for entry in entries {
        let entry = entry
            .map_err(|e| ChainError::WalletError(format!("Failed to read directory entry: {}", e)))?;

        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                wallets.push(filename.to_string());
            }
        }
    }

    wallets.sort();
    Ok(wallets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_wallet_creation() {
        let wallet = Wallet::new(Some("test".to_string())).unwrap();
        assert_eq!(wallet.name, Some("test".to_string()));
        assert!(!wallet.address.is_empty());
        assert!(!wallet.secret_key_hex.is_empty());
    }

    #[test]
    fn test_wallet_keypair_recovery() {
        let wallet = Wallet::new(None).unwrap();
        let keypair = wallet.get_keypair().unwrap();
        assert_eq!(wallet.address, keypair.address());
    }

    #[test]
    fn test_wallet_save_and_load() {
        let temp_dir = std::env::temp_dir();
        let wallet_path = temp_dir.join("test_wallet.json");

        // Clean up if exists
        let _ = fs::remove_file(&wallet_path);

        // Create and save
        let wallet = Wallet::new(Some("test_save".to_string())).unwrap();
        wallet.save(&wallet_path).unwrap();

        // Load and verify
        let loaded = Wallet::load(&wallet_path).unwrap();
        assert_eq!(wallet.address, loaded.address);
        assert_eq!(wallet.secret_key_hex, loaded.secret_key_hex);
        assert_eq!(wallet.name, loaded.name);

        // Cleanup
        fs::remove_file(&wallet_path).unwrap();
    }
}
