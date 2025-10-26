//! Core structures for the siertrichain blockchain.
//! Defines Block, Blockchain, and the Triangle UTXO State.

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use chrono::{Utc, DateTime};
use sha2::{Digest, Sha256};
use crate::geometry::Triangle;
use crate::transaction::{Transaction, SubdivisionTx, CoinbaseTx};
use crate::error::ChainError; 

/// Canonical hash type for blocks, transactions, and triangles.
pub type Sha256Hash = String;
/// Block height/index.
pub type BlockHeight = u64;

// ----------------------------------------------------------------------------
// 1.5 Fractal State Management
// ----------------------------------------------------------------------------

/// Manages the canonical set of all currently valid (unspent) triangles (UTXO set).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TriangleState {
    pub utxo_set: HashMap<Sha256Hash, Triangle>,
}

impl TriangleState {
    pub fn new() -> Self {
        TriangleState {
            utxo_set: HashMap::new(),
        }
    }

    pub fn init_genesis(&mut self) {
        let genesis_triangle = Triangle::genesis();
        let hash = genesis_triangle.hash();
        self.utxo_set.insert(hash, genesis_triangle);
    }
    
    pub fn contains(&self, hash: &str) -> bool {
        self.utxo_set.contains_key(hash)
    }

    pub fn count(&self) -> usize {
        self.utxo_set.len()
    }
    
    pub fn get_triangle(&self, hash: &str) -> Result<&Triangle, ChainError> {
        self.utxo_set.get(hash).ok_or(ChainError::TriangleNotFound(hash.to_string()))
    }
    
    fn apply_subdivision(&mut self, tx: &SubdivisionTx) -> Result<(), ChainError> {
        if self.utxo_set.remove(&tx.parent_hash).is_none() {
            return Err(ChainError::TriangleNotFound(format!("Failed to consume parent: {}", tx.parent_hash)));
        }
        
        for child in tx.children.iter() {
            self.utxo_set.insert(child.hash(), child.clone());
        }
        
        Ok(())
    }

    fn apply_coinbase(&mut self, _tx: &CoinbaseTx) {
        // Implementation note: No new UTXO is created for the simple reward model.
    }
}

// ----------------------------------------------------------------------------
// 2.4 Merkle Tree Implementation
// ----------------------------------------------------------------------------

/// Helper function to perform a double-SHA256 hash.
fn double_sha256(data: &str) -> Sha256Hash {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    let first_hash = hasher.finalize_reset();
    
    hasher.update(&first_hash);
    format!("{:x}", hasher.finalize())
}

/// Builds a Merkle Tree from a list of transaction hashes and returns the Merkle Root.
pub fn build_merkle_tree(hashes: Vec<Sha256Hash>) -> Sha256Hash {
    if hashes.is_empty() {
        return "4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e".to_string();
    }
    
    if hashes.len() == 1 {
        let hash = &hashes[0];
        return double_sha256(&format!("{}{}", hash, hash));
    }
    
    let mut layer = hashes;

    while layer.len() > 1 {
        let mut next_layer = Vec::new();
        let mut i = 0;
        
        while i < layer.len() {
            let left = &layer[i];
            
            let right = if i + 1 < layer.len() {
                &layer[i + 1]
            } else {
                left 
            };

            let combined = format!("{}{}", left, right);
            let combined_hash = double_sha256(&combined);
            next_layer.push(combined_hash);
            
            i += 2;
        }
        
        layer = next_layer;
    }

    layer[0].clone()
}


// ----------------------------------------------------------------------------
// 2.1 Block Structure
// ----------------------------------------------------------------------------

/// The core unit of the siertrichain ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub height: BlockHeight,
    pub timestamp: DateTime<Utc>,
    pub previous_hash: Sha256Hash,
    pub merkle_root: Sha256Hash,
    pub difficulty: u64,
    pub nonce: u64,
    pub transactions: Vec<Transaction>,
    pub hash: Sha256Hash,
}

impl Block {
    pub fn new(height: BlockHeight, previous_hash: Sha256Hash, difficulty: u64, transactions: Vec<Transaction>) -> Self {
        Block {
            height,
            timestamp: Utc::now(),
            previous_hash,
            merkle_root: Self::calculate_merkle_root(&transactions), 
            difficulty,
            nonce: 0, 
            transactions,
            hash: "".to_string(), 
        }
    }

    /// Calculates the block's canonical hash for the Proof-of-Work.
    pub fn calculate_hash(&self) -> Sha256Hash {
        let block_data = format!(
            "{}{}{}{}{}{}",
            self.height,
            self.timestamp.to_rfc3339(),
            self.previous_hash,
            self.merkle_root,
            self.difficulty,
            self.nonce,
        );
        let mut hasher = Sha256::new();
        hasher.update(block_data.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Calculates the Merkle Root of the block's transactions.
    fn calculate_merkle_root(transactions: &[Transaction]) -> Sha256Hash {
        let tx_hashes: Vec<Sha256Hash> = transactions.iter()
            .map(|tx| tx.hash())
            .collect();
        
        build_merkle_tree(tx_hashes)
    }
}


// ----------------------------------------------------------------------------
// 2.2 Blockchain Implementation
// ----------------------------------------------------------------------------

/// The target time (in seconds) we aim for between blocks.
const TARGET_BLOCK_TIME_SECONDS: i64 = 10;
/// The number of blocks to look back for difficulty adjustment.
const DIFFICULTY_ADJUSTMENT_WINDOW: BlockHeight = 10;

/// The main chain data structure.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Blockchain {
    pub blocks: Vec<Block>,
    pub state: TriangleState,
    pub difficulty: u64,
}

impl Blockchain {
    pub fn new() -> Self {
        let mut chain = Blockchain {
            blocks: Vec::new(),
            state: TriangleState::new(),
            difficulty: 2, 
        };
        chain.add_genesis_block();
        chain
    }
    
    fn add_genesis_block(&mut self) {
        self.state.init_genesis();

        let headline = "Optimism rises for Thursday summit between Trump and Xi";
        let genesis_block_hash = "0000000000000000000000000000000000000000000000000000000000000000".to_string();
        
        let mut genesis_block = Block::new(
            0,
            genesis_block_hash.clone(),
            self.difficulty,
            vec![],
        );
        
        genesis_block.nonce = 42;
        genesis_block.hash = genesis_block_hash;
        
        self.blocks.push(genesis_block);
        
        println!("🔺 Genesis Block Created");
        println!("   Headline: \"{}\"", headline);
        println!("   Date: October 26, 2025");
        println!("   Hash: {}", self.blocks[0].hash);
    }
    
    /// Adjusts the mining difficulty based on the time taken for the last adjustment window.
    pub fn adjust_difficulty(&mut self) {
        let current_height = self.blocks.last().map_or(0, |b| b.height);
        
        if current_height == 0 || current_height % DIFFICULTY_ADJUSTMENT_WINDOW != 0 {
            return;
        }

        let start_index = (current_height - DIFFICULTY_ADJUSTMENT_WINDOW) as usize;
        let end_index = current_height as usize;

        if start_index >= self.blocks.len() {
            return; 
        }

        let first_block = &self.blocks[start_index];
        let last_block = &self.blocks[end_index];
        
        let actual_time_taken = last_block.timestamp.signed_duration_since(first_block.timestamp).num_seconds();
        let expected_time_taken = DIFFICULTY_ADJUSTMENT_WINDOW as i64 * TARGET_BLOCK_TIME_SECONDS;
        
        let time_ratio = actual_time_taken as f64 / expected_time_taken as f64;
        
        let mut new_difficulty = self.difficulty as f64;

        if time_ratio < 0.25 { 
            new_difficulty *= 1.25; 
        } else if time_ratio > 4.0 { 
            new_difficulty *= 0.75; 
        } else if time_ratio < 1.0 { 
            new_difficulty *= 1.0 + (1.0 - time_ratio) / 4.0;
        } else if time_ratio > 1.0 { 
            new_difficulty *= 1.0 - (time_ratio - 1.0) / 4.0;
        }
        
        self.difficulty = new_difficulty.round().max(1.0) as u64;
    }
    
    // ------------------------------------------------------------------------
    // 2.6 Block Validation and Application
    // ------------------------------------------------------------------------

    /// Validates a potential new block against the current state of the blockchain.
    pub fn validate_block(&self, new_block: &Block) -> Result<(), ChainError> {
        let last_block = self.blocks.last()
            .ok_or(ChainError::InvalidBlockLinkage)?; 
        
        // 1. Check Linkage (Height and Previous Hash)
        if new_block.height != last_block.height + 1 {
            return Err(ChainError::InvalidBlockLinkage);
        }
        if new_block.previous_hash != last_block.hash {
            return Err(ChainError::InvalidBlockLinkage);
        }
        
        // 2. Check Proof-of-Work (PoW)
        if new_block.calculate_hash() != new_block.hash {
            return Err(ChainError::InvalidProofOfWork);
        }
        
        // 3. Check Merkle Root Integrity
        let calculated_merkle_root = Block::calculate_merkle_root(&new_block.transactions);
        if new_block.merkle_root != calculated_merkle_root {
            return Err(ChainError::InvalidMerkleRoot);
        }
        
        // 4. Validate Transactions (against the current UTXO set and self-spends)
        let mut temp_state = self.state.clone(); 
        
        for tx in new_block.transactions.iter() {
            tx.validate(&temp_state)?;

            match tx {
                Transaction::Subdivision(sub_tx) => {
                    temp_state.apply_subdivision(sub_tx)?;
                },
                Transaction::Coinbase(cb_tx) => {
                    temp_state.apply_coinbase(cb_tx);
                }
            }
        }
        
        Ok(())
    }
    
    /// Adds a fully validated block to the chain and updates the UTXO state.
    pub fn apply_block(&mut self, valid_block: Block) -> Result<(), ChainError> {
        self.validate_block(&valid_block)?; 
        
        for tx in valid_block.transactions.iter() {
            match tx {
                Transaction::Subdivision(sub_tx) => {
                    self.state.apply_subdivision(sub_tx)?;
                },
                Transaction::Coinbase(cb_tx) => {
                    self.state.apply_coinbase(cb_tx);
                }
            }
        }
        
        self.blocks.push(valid_block);
        self.adjust_difficulty();
        
        Ok(())
    }
}


// ----------------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------------


// ---------------------------------------------------
// Tests
// ---------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Triangle;
    use crate::transaction::{SubdivisionTx, Transaction};
    use crate::crypto::KeyPair;
    use chrono::Duration;
    use crate::miner::mine_block;

    fn create_dummy_transaction() -> Transaction {
        let t = Triangle::genesis();
        let children_vec = t.subdivide();
        let children: [Triangle; 3] = [
            children_vec[0].clone(),
            children_vec[1].clone(),
            children_vec[2].clone(),
        ];
        
        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();
        
        let mut tx = SubdivisionTx::new(
            t.hash(),
            children,
            address,
            100,
            1,
        );
        
        // Sign the transaction
        let message = tx.signable_message();
        let signature = keypair.sign(&message).unwrap();
        let public_key = keypair.public_key.serialize().to_vec();
        tx.sign(signature, public_key);
        
        Transaction::Subdivision(tx)
    }

    // Static hashes for Merkle testing
    const HASH_1: &str = "6decb2358ed4706605442064120ed5d4f5841d0dcbe013e76b64922701518a5e";
    const HASH_2: &str = "c38221a8c0d67f46e03e75af95b7f48bc3ee42f52e00face049361d2fddcb42c";
    const HASH_3: &str = "08a9f3e09880f0d2c0e86b2450410641f05785f7e7f603227493a776105f2385";

    #[test]
    fn test_merkle_tree_empty() {
        let hashes = vec![];
        let root = build_merkle_tree(hashes);
        assert_eq!(root, "4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e");
    }

    #[test]
    fn test_merkle_tree_single() {
        let unique_hash_string = "TEST_SINGLE_MERKLE_HASH".to_string();
        let hashes = vec![unique_hash_string.clone()];
        let root = build_merkle_tree(hashes);
        const EXPECTED_SINGLE_ROOT: &str = "c51658d947a0a452f29333da947126f54da2f71adb39683b249dfbaf5efa1916";
        assert_eq!(root, EXPECTED_SINGLE_ROOT);
    }

    #[test]
    fn test_merkle_tree_even() {
        let h1 = HASH_1.to_string();
        let h2 = HASH_2.to_string();
        let hashes = vec![h1.clone(), h2.clone()];
        let root = build_merkle_tree(hashes);
        let expected_root = double_sha256(&format!("{}{}", h1, h2));
        assert_eq!(root, expected_root);
    }

    #[test]
    fn test_merkle_tree_odd() {
        let h1 = HASH_1.to_string();
        let h2 = HASH_2.to_string();
        let h3 = HASH_3.to_string();
        let hashes = vec![h1.clone(), h2.clone(), h3.clone()];
        let root = build_merkle_tree(hashes);

        let h12 = double_sha256(&format!("{}{}", h1, h2));
        let h33 = double_sha256(&format!("{}{}", h3, h3));
        let expected_root = double_sha256(&format!("{}{}", h12, h33));

        assert_eq!(root, expected_root);
    }

    #[test]
    fn test_block_merkle_root_calculation() {
        let tx1 = create_dummy_transaction();
        let tx2 = create_dummy_transaction();
        let tx_hashes = vec![tx1.hash(), tx2.hash()];

        let block = Block::new(
            1,
            "prev_hash".to_string(),
            10,
            vec![tx1, tx2]
        );

        let expected_root = build_merkle_tree(tx_hashes);
        assert_eq!(block.merkle_root, expected_root);
    }

    // Difficulty adjustment tests
    fn create_mock_block(height: u64, timestamp_offset: i64) -> Block {
        Block {
            height,
            timestamp: Utc::now() + Duration::seconds(timestamp_offset),
            previous_hash: "mock_prev_hash".to_string(),
            merkle_root: "mock_merkle_root".to_string(),
            difficulty: 2,
            nonce: 0,
            transactions: vec![],
            hash: "".to_string(),
        }
    }

    #[test]
    fn test_difficulty_adjustment_no_change() {
        let _chain = Blockchain::new();
        let mut current_time_offset = 10;
        let mut mock_blocks = Vec::new();

        for i in 1..=DIFFICULTY_ADJUSTMENT_WINDOW {
            mock_blocks.push(create_mock_block(i, current_time_offset));
            current_time_offset += 10;
        }

        let mut chain_with_history = Blockchain::new();
        chain_with_history.difficulty = 10;
        chain_with_history.blocks.extend(mock_blocks);

        chain_with_history.adjust_difficulty();
        assert_eq!(chain_with_history.difficulty, 10);
    }

    #[test]
    fn test_difficulty_adjustment_increase() {
        let _chain = Blockchain::new();

        let mut current_time_offset = 1;
        let mut mock_blocks = Vec::new();
        for i in 1..=DIFFICULTY_ADJUSTMENT_WINDOW {
            mock_blocks.push(create_mock_block(i, current_time_offset));
            current_time_offset += 1;
        }

        let mut chain_with_history = Blockchain::new();
        chain_with_history.difficulty = 10;
        chain_with_history.blocks.extend(mock_blocks);

        chain_with_history.adjust_difficulty();
        assert_eq!(chain_with_history.difficulty, 13, "Difficulty should increase with a fast time ratio (0.1 < 0.25).");
    }

    #[test]
    fn test_difficulty_adjustment_decrease() {
        let _chain = Blockchain::new();

        let mut current_time_offset = 50;
        let mut mock_blocks = Vec::new();
        for i in 1..=DIFFICULTY_ADJUSTMENT_WINDOW {
            mock_blocks.push(create_mock_block(i, current_time_offset));
            current_time_offset += 50;
        }

        let mut chain_with_history = Blockchain::new();
        chain_with_history.difficulty = 10;
        chain_with_history.blocks.extend(mock_blocks);

        chain_with_history.adjust_difficulty();
        assert_eq!(chain_with_history.difficulty, 8, "Difficulty should decrease with a slow time ratio (5.0 > 4.0).");
    }

    fn create_valid_next_block(chain: &Blockchain, transactions: Vec<Transaction>) -> Block {
        let last_block = chain.blocks.last().unwrap();

        let block_template = Block::new(
            last_block.height + 1,
            last_block.hash.clone(),
            chain.difficulty,
            transactions,
        );

        mine_block(block_template).expect("Mining should succeed for low difficulty")
    }

    #[test]
    fn test_block_validation_success() {
        let chain = Blockchain::new();

        let subdivision_tx = match create_dummy_transaction() {
            Transaction::Subdivision(tx) => tx,
            _ => panic!("Expected subdivision"),
        };
        let transactions = vec![
            Transaction::Coinbase(CoinbaseTx { reward_area: 1000, beneficiary_address: "miner".to_string() }),
            Transaction::Subdivision(subdivision_tx.clone()),
        ];

        let valid_block = create_valid_next_block(&chain, transactions);

        assert!(chain.validate_block(&valid_block).is_ok());
    }

    #[test]
    fn test_block_validation_failure_pow() {
        let chain = Blockchain::new();
        let mut block = create_valid_next_block(&chain, vec![]);

        block.hash = "f00f00f00f00f00f00f00f00f00f00f00f00f00f00f00f00f00f00f00f00f00f".to_string();

        match chain.validate_block(&block) {
            Err(ChainError::InvalidProofOfWork) => assert!(true),
            _ => panic!("Expected InvalidProofOfWork"),
        }
    }

    #[test]
    fn test_block_validation_failure_linkage() {
        let chain = Blockchain::new();
        let mut block = create_valid_next_block(&chain, vec![]);

        block.previous_hash = "bad_hash".to_string();

        match chain.validate_block(&block) {
            Err(ChainError::InvalidBlockLinkage) => assert!(true),
            _ => panic!("Expected InvalidBlockLinkage"),
        }
    }

    #[test]
    fn test_block_validation_double_spend_in_block() {
        let chain = Blockchain::new();
        let tx1 = create_dummy_transaction();
        let tx2 = create_dummy_transaction();

        let transactions = vec![tx1, tx2];
        let block = create_valid_next_block(&chain, transactions);

        match chain.validate_block(&block) {
            Err(ChainError::TriangleNotFound(_)) => assert!(true),
            _ => panic!("Expected TriangleNotFound error for double spend check."),
        }
    }

    #[test]
    fn test_apply_block_updates_state() {
        let mut chain = Blockchain::new();
        let genesis_utxo_hash = chain.state.utxo_set.keys().next().unwrap().clone();

        let subdivision_tx = match create_dummy_transaction() {
            Transaction::Subdivision(tx) => tx,
            _ => panic!(),
        };
        let child_hashes: Vec<String> = subdivision_tx.children.iter().map(|t| t.hash()).collect();

        let transactions = vec![Transaction::Subdivision(subdivision_tx)];
        let valid_block = create_valid_next_block(&chain, transactions);

        let initial_utxo_count = chain.state.count();
        assert!(chain.apply_block(valid_block).is_ok());

        assert_eq!(chain.blocks.len(), 2);
        assert_eq!(chain.blocks.last().unwrap().height, 1);

        assert_eq!(chain.state.count(), initial_utxo_count - 1 + 3);
        assert!(!chain.state.contains(&genesis_utxo_hash));
        assert!(chain.state.contains(&child_hashes[0]));
    }
}
