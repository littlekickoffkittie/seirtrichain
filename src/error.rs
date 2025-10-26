//! Error types for siertrichain

use std::fmt;

#[derive(Debug, Clone)]
pub enum ChainError {
    InvalidBlockLinkage,
    NetworkError(String),
    DatabaseError(String),
    InvalidProofOfWork,
    InvalidMerkleRoot,
    InvalidTransaction(String),
    TriangleNotFound(String),
    CryptoError(String),
}

impl fmt::Display for ChainError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ChainError::InvalidBlockLinkage => write!(f, "Invalid block linkage"),
            ChainError::InvalidProofOfWork => write!(f, "Invalid proof of work"),
            ChainError::InvalidMerkleRoot => write!(f, "Invalid Merkle root"),
            ChainError::InvalidTransaction(msg) => write!(f, "Invalid transaction: {}", msg),
            ChainError::TriangleNotFound(msg) => write!(f, "Triangle not found: {}", msg),
            ChainError::CryptoError(msg) => write!(f, "Cryptographic error: {}", msg),
            ChainError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            ChainError::NetworkError(msg) => write!(f, "Network error: {}", msg),
        }
    }
}

impl std::error::Error for ChainError {}
