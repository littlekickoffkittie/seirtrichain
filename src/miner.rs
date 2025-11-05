//! Proof-of-Work (PoW) implementation for siertrichain.

use crate::blockchain::{Block, Sha256Hash};
use crate::error::ChainError;

/// Checks if a hash meets the required difficulty target.
/// The difficulty is the required number of leading zeros in the hash.
pub fn is_hash_valid(hash: &Sha256Hash, difficulty: u64) -> bool {
    let required_prefix = "0".repeat(difficulty as usize);
    hex::encode(hash).starts_with(&required_prefix)
}

/// Mines a new block by searching for a nonce that satisfies the current difficulty.
pub fn mine_block(mut block: Block) -> Result<Block, ChainError> {
    let difficulty = block.header.difficulty;
    let mut nonce: u64 = 0;
    
    loop {
        block.header.nonce = nonce;
        let hash = block.calculate_hash();
        
        if is_hash_valid(&hash, difficulty) {
            block.hash = hash;
            return Ok(block);
        }

        nonce = nonce.checked_add(1).ok_or(ChainError::InvalidProofOfWork)?; 
    }
}
