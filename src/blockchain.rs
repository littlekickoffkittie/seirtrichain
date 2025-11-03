//! Core blockchain implementation for siertrichain

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use crate::geometry::{Triangle, Point};
use crate::transaction::{Transaction, SubdivisionTx, CoinbaseTx};
use crate::error::ChainError;
use chrono::Utc;

pub type Sha256Hash = String;
pub type BlockHeight = u64;

/// The genesis triangle - the root of all fractals
pub fn genesis_triangle() -> Triangle {
    Triangle::new(
        Point { x: 0.0, y: 0.0 },
        Point { x: 1.0, y: 0.0 },
        Point { x: 0.5, y: 0.866025403784 },
        None,
        "genesis_owner".to_string(),
    )
}

/// Manages the canonical set of all currently valid (unspent) triangles (UTXO set).
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct TriangleState {
    pub utxo_set: HashMap<Sha256Hash, Triangle>,
}

impl TriangleState {
    pub fn new() -> Self {
        TriangleState {
            utxo_set: HashMap::new(),
        }
    }

    pub fn count(&self) -> usize {
        self.utxo_set.len()
    }

    /// Apply a subdivision transaction to the state
    pub fn apply_subdivision(&mut self, tx: &SubdivisionTx) -> Result<(), ChainError> {
        if !self.utxo_set.contains_key(&tx.parent_hash) {
            return Err(ChainError::TriangleNotFound(format!(
                "Parent triangle {} not found",
                tx.parent_hash
            )));
        }

        self.utxo_set.remove(&tx.parent_hash);

        for child in &tx.children {
            let child_hash = child.hash();
            self.utxo_set.insert(child_hash, child.clone());
        }

        Ok(())
    }

    /// Apply a coinbase transaction (no-op for state, just validation)
    pub fn apply_coinbase(&mut self, _tx: &CoinbaseTx) {
        // Coinbase doesn't modify UTXO set
    }
}

/// Represents a block header with metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlockHeader {
    pub height: BlockHeight,
    pub previous_hash: Sha256Hash,
    pub timestamp: i64,
    pub difficulty: u64,
    pub nonce: u64,
    pub merkle_root: Sha256Hash,
}

/// A block in the blockchain
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub hash: Sha256Hash,
    pub height: BlockHeight,
    pub previous_hash: Sha256Hash,
    pub timestamp: i64,
    pub difficulty: u64,
    pub nonce: u64,
    pub merkle_root: Sha256Hash,
    pub transactions: Vec<Transaction>,
}

impl Block {
    pub fn new(height: BlockHeight, previous_hash: Sha256Hash, difficulty: u64, transactions: Vec<Transaction>) -> Self {
        let timestamp = Utc::now().timestamp();
        let merkle_root = Self::calculate_merkle_root(&transactions);
        
        let header = BlockHeader {
            height,
            previous_hash: previous_hash.clone(),
            timestamp,
            difficulty,
            nonce: 0,
            merkle_root: merkle_root.clone(),
        };

        Block {
            header: header.clone(),
            hash: String::new(),
            height,
            previous_hash,
            timestamp,
            difficulty,
            nonce: 0,
            merkle_root,
            transactions,
        }
    }

    pub fn calculate_hash(&self) -> String {
        let header_data = format!(
            "{}{}{}{}{}{}",
            self.height,
            self.previous_hash,
            self.timestamp,
            self.difficulty,
            self.nonce,
            self.merkle_root
        );

        let mut hasher = Sha256::new();
        hasher.update(header_data.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn calculate_merkle_root(transactions: &[Transaction]) -> Sha256Hash {
        if transactions.is_empty() {
            return String::from("0000000000000000000000000000000000000000000000000000000000000000");
        }

        let mut hashes: Vec<String> = transactions.iter().map(|tx| tx.hash()).collect();

        while hashes.len() > 1 {
            let mut next_level = Vec::new();

            for chunk in hashes.chunks(2) {
                let combined = if chunk.len() == 2 {
                    format!("{}{}", chunk[0], chunk[1])
                } else {
                    format!("{}{}", chunk[0], chunk[0])
                };

                let mut hasher = Sha256::new();
                hasher.update(combined.as_bytes());
                next_level.push(format!("{:x}", hasher.finalize()));
            }

            hashes = next_level;
        }

        hashes[0].clone()
    }

    pub fn verify_proof_of_work(&self) -> bool {
        let leading_zeros = "0".repeat(self.difficulty as usize);
        self.hash.starts_with(&leading_zeros)
    }
}

/// Transaction pool for pending (unconfirmed) transactions
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Mempool {
    /// Pending transactions indexed by their hash
    transactions: HashMap<Sha256Hash, Transaction>,
}

impl Mempool {
    /// Maximum number of transactions in mempool (to prevent DoS)
    const MAX_TRANSACTIONS: usize = 10000;

    /// Maximum transactions per address to prevent spam
    const MAX_PER_ADDRESS: usize = 100;

    pub fn new() -> Self {
        Mempool {
            transactions: HashMap::new(),
        }
    }

    /// Add a transaction to the mempool with validation
    pub fn add_transaction(&mut self, tx: Transaction) -> Result<(), ChainError> {
        let tx_hash = tx.hash();

        // Check if transaction already exists
        if self.transactions.contains_key(&tx_hash) {
            return Err(ChainError::InvalidTransaction(
                "Transaction already in mempool".to_string()
            ));
        }

        // Validate transaction before adding to mempool
        match &tx {
            Transaction::Transfer(transfer_tx) => {
                // Validate signature before adding
                transfer_tx.validate()?;
            },
            Transaction::Coinbase(_) => {
                return Err(ChainError::InvalidTransaction(
                    "Coinbase transactions cannot be added to mempool".to_string()
                ));
            },
            Transaction::Subdivision(_) => {
                // Subdivision validation requires state, so we skip it here
                // It will be validated during mining or in validate_and_prune
            }
        }

        // Check per-address limit to prevent spam
        let sender_address = match &tx {
            Transaction::Transfer(t) => Some(&t.sender),
            Transaction::Subdivision(s) => Some(&s.owner_address),
            Transaction::Coinbase(_) => None,
        };

        if let Some(sender) = sender_address {
            let count = self.transactions.values()
                .filter(|t| match t {
                    Transaction::Transfer(t) => &t.sender == sender,
                    Transaction::Subdivision(s) => &s.owner_address == sender,
                    _ => false,
                })
                .count();

            if count >= Self::MAX_PER_ADDRESS {
                return Err(ChainError::InvalidTransaction(
                    format!("Address has reached maximum mempool limit of {}", Self::MAX_PER_ADDRESS)
                ));
            }
        }

        // If mempool is full, evict lowest fee transaction
        if self.transactions.len() >= Self::MAX_TRANSACTIONS {
            self.evict_lowest_fee_transaction()?;
        }

        self.transactions.insert(tx_hash, tx);
        Ok(())
    }

    /// Evict the transaction with the lowest fee to make room for new ones
    fn evict_lowest_fee_transaction(&mut self) -> Result<(), ChainError> {
        if self.transactions.is_empty() {
            return Ok(());
        }

        // Find transaction with lowest fee
        let mut lowest_fee = u64::MAX;
        let mut lowest_hash: Option<String> = None;

        for (hash, tx) in &self.transactions {
            let fee = match tx {
                Transaction::Transfer(t) => t.fee,
                Transaction::Subdivision(_) => 0, // Subdivisions don't have fees
                Transaction::Coinbase(_) => 0,
            };

            if fee < lowest_fee {
                lowest_fee = fee;
                lowest_hash = Some(hash.clone());
            }
        }

        if let Some(hash) = lowest_hash {
            self.transactions.remove(&hash);
        }

        Ok(())
    }

    /// Remove a transaction from the mempool
    pub fn remove_transaction(&mut self, tx_hash: &str) -> Option<Transaction> {
        self.transactions.remove(tx_hash)
    }

    /// Get all transactions currently in the mempool
    pub fn get_all_transactions(&self) -> Vec<Transaction> {
        self.transactions.values().cloned().collect()
    }

    /// Get a specific transaction by hash
    pub fn get_transaction(&self, tx_hash: &str) -> Option<&Transaction> {
        self.transactions.get(tx_hash)
    }

    /// Remove multiple transactions (e.g., after they're included in a block)
    pub fn remove_transactions(&mut self, tx_hashes: &[String]) {
        for hash in tx_hashes {
            self.transactions.remove(hash);
        }
    }

    /// Clear all transactions from the mempool
    pub fn clear(&mut self) {
        self.transactions.clear();
    }

    /// Get the number of pending transactions
    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    /// Check if mempool is empty
    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }

    /// Validate all transactions in mempool against current state
    /// Removes invalid transactions and returns count of removed transactions
    pub fn validate_and_prune(&mut self, state: &TriangleState) -> usize {
        let mut to_remove = Vec::new();

        for (hash, tx) in self.transactions.iter() {
            let is_valid = match tx {
                Transaction::Subdivision(sub_tx) => {
                    // Check if parent exists in UTXO set
                    state.utxo_set.contains_key(&sub_tx.parent_hash) &&
                    sub_tx.validate(state).is_ok()
                },
                Transaction::Transfer(transfer_tx) => {
                    // Check if input exists in UTXO set
                    state.utxo_set.contains_key(&transfer_tx.input_hash) &&
                    transfer_tx.validate().is_ok()
                },
                Transaction::Coinbase(_) => {
                    // Coinbase transactions shouldn't be in mempool
                    false
                }
            };

            if !is_valid {
                to_remove.push(hash.clone());
            }
        }

        let removed_count = to_remove.len();
        for hash in to_remove {
            self.transactions.remove(&hash);
        }

        removed_count
    }
}

/// The blockchain itself
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Blockchain {
    pub blocks: Vec<Block>,
    pub block_index: HashMap<Sha256Hash, Block>,
    pub state: TriangleState,
    pub difficulty: u64,
    pub mempool: Mempool,
}

const DIFFICULTY_ADJUSTMENT_WINDOW: BlockHeight = 10;
const TARGET_BLOCK_TIME_SECONDS: i64 = 60;

impl Blockchain {
    pub fn new() -> Self {
        let mut state = TriangleState::new();
        let genesis = genesis_triangle();
        let genesis_hash = genesis.hash();
        state.utxo_set.insert(genesis_hash.clone(), genesis);

        let genesis_block = Block {
            header: BlockHeader {
                height: 0,
                previous_hash: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                timestamp: Utc::now().timestamp(),
                difficulty: 2,
                nonce: 0,
                merkle_root: "genesis".to_string(),
            },
            hash: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            height: 0,
            previous_hash: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            timestamp: Utc::now().timestamp(),
            difficulty: 2,
            nonce: 0,
            merkle_root: "genesis".to_string(),
            transactions: vec![],
        };

        let mut block_index = HashMap::new();
        block_index.insert(genesis_block.hash.clone(), genesis_block.clone());

        Blockchain {
            blocks: vec![genesis_block],
            block_index,
            state,
            difficulty: 2,
            mempool: Mempool::new(),
        }
    }

    pub fn validate_block(&self, block: &Block) -> Result<(), ChainError> {
        if self.blocks.is_empty() {
            return Err(ChainError::InvalidBlockLinkage);
        }

        let last_block = self.blocks.last().unwrap();

        if block.height != last_block.height + 1 {
            return Err(ChainError::InvalidBlockLinkage);
        }

        if block.previous_hash != last_block.hash {
            return Err(ChainError::InvalidBlockLinkage);
        }

        if !block.verify_proof_of_work() {
            return Err(ChainError::InvalidProofOfWork);
        }

        let calculated_merkle = Block::calculate_merkle_root(&block.transactions);
        if block.merkle_root != calculated_merkle {
            return Err(ChainError::InvalidMerkleRoot);
        }

        // Validate coinbase transaction rules
        let mut coinbase_count = 0;
        for (i, tx) in block.transactions.iter().enumerate() {
            if let Transaction::Coinbase(_) = tx {
                coinbase_count += 1;
                // Coinbase must be the first transaction
                if i != 0 {
                    return Err(ChainError::InvalidTransaction(
                        "Coinbase transaction must be the first transaction in the block".to_string()
                    ));
                }
            }
        }

        // Exactly one coinbase transaction per block (or zero for genesis)
        if block.height > 0 && coinbase_count != 1 {
            return Err(ChainError::InvalidTransaction(
                format!("Block must contain exactly one coinbase transaction, found {}", coinbase_count)
            ));
        }

        for tx in block.transactions.iter() {
            match tx {
                Transaction::Subdivision(tx) => {
                    if !self.state.utxo_set.contains_key(&tx.parent_hash) {
                        return Err(ChainError::InvalidTransaction(
                            format!("Parent triangle {} not in UTXO set", tx.parent_hash)
                        ));
                    }
                    tx.validate(&self.state)?;
                },
                Transaction::Coinbase(cb_tx) => {
                    cb_tx.validate()?;
                },
                Transaction::Transfer(tx) => {
                    if !self.state.utxo_set.contains_key(&tx.input_hash) {
                        return Err(ChainError::InvalidTransaction(
                            format!("Transfer input {} not in UTXO set", tx.input_hash)
                        ));
                    }
                    tx.validate()?;
                },
            }
        }

        Ok(())
    }

    pub fn apply_block(&mut self, valid_block: Block) -> Result<(), ChainError> {
        self.validate_block(&valid_block)?;

        // Collect transaction hashes before applying
        let tx_hashes: Vec<String> = valid_block.transactions.iter()
            .map(|tx| tx.hash())
            .collect();

        for tx in valid_block.transactions.iter() {
            match tx {
                Transaction::Subdivision(sub_tx) => {
                    self.state.apply_subdivision(sub_tx)?;
                },
                Transaction::Coinbase(cb_tx) => {
                    self.state.apply_coinbase(cb_tx);
                },
                Transaction::Transfer(tx) => {
                    let mut triangle = self.state.utxo_set.remove(&tx.input_hash)
                        .ok_or_else(|| ChainError::TriangleNotFound(
                            format!("Transfer input {} missing from UTXO set", tx.input_hash)
                        ))?;
                    triangle.owner = tx.new_owner.clone();
                    let new_hash = format!("{:x}", Sha256::digest(
                        format!("{}:{}", tx.input_hash, tx.new_owner).as_bytes()
                    ));
                    self.state.utxo_set.insert(new_hash, triangle);
                }
            }
        }

        self.blocks.push(valid_block.clone());
        self.block_index.insert(valid_block.hash.clone(), valid_block);
        self.adjust_difficulty();

        // Remove confirmed transactions from mempool
        self.mempool.remove_transactions(&tx_hashes);

        // Prune any now-invalid transactions from mempool
        self.mempool.validate_and_prune(&self.state);

        Ok(())
    }

    fn adjust_difficulty(&mut self) {
        if self.blocks.len() < DIFFICULTY_ADJUSTMENT_WINDOW as usize {
            return;
        }

        let window_start = self.blocks.len() - DIFFICULTY_ADJUSTMENT_WINDOW as usize;
        let first_block = &self.blocks[window_start];
        let last_block = self.blocks.last().unwrap();

        let time_taken = last_block.timestamp - first_block.timestamp;

        // Guard against zero or negative time (e.g., in testing scenarios)
        if time_taken <= 0 {
            return;
        }

        let expected_time = TARGET_BLOCK_TIME_SECONDS * DIFFICULTY_ADJUSTMENT_WINDOW as i64;

        // Add max difficulty cap to prevent runaway difficulty
        const MAX_DIFFICULTY: u64 = 256;

        if time_taken < expected_time / 2 {
            self.difficulty = (self.difficulty + 1).min(MAX_DIFFICULTY);
        } else if time_taken > expected_time * 2 && self.difficulty > 1 {
            self.difficulty -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Triangle;
    use crate::transaction::{SubdivisionTx, Transaction};
    use crate::crypto::KeyPair;

    #[test]
    fn test_genesis_triangle_is_canonical() {
        let genesis = genesis_triangle();
        assert_eq!(genesis.a.x, 0.0);
        assert_eq!(genesis.a.y, 0.0);
        assert_eq!(genesis.b.x, 1.0);
        assert_eq!(genesis.c.x, 0.5);
        assert!((genesis.c.y - 0.866025403784).abs() < 1e-10);
    }

    #[test]
    fn test_block_merkle_root_calculation() {
        let coinbase = CoinbaseTx {
            reward_area: 1000,
            beneficiary_address: "test".to_string(),
        };
        let transactions = vec![Transaction::Coinbase(coinbase)];
        let merkle = Block::calculate_merkle_root(&transactions);
        assert!(!merkle.is_empty());
    }

    #[test]
    fn test_merkle_tree_empty() {
        let root = Block::calculate_merkle_root(&[]);
        assert_eq!(root, "0000000000000000000000000000000000000000000000000000000000000000");
    }

    #[test]
    fn test_merkle_tree_single() {
        let coinbase = CoinbaseTx {
            reward_area: 1000,
            beneficiary_address: "miner".to_string(),
        };
        let txs = vec![Transaction::Coinbase(coinbase)];
        let root = Block::calculate_merkle_root(&txs);
        assert_eq!(root.len(), 64);
    }

    #[test]
    fn test_merkle_tree_even() {
        let tx1 = Transaction::Coinbase(CoinbaseTx {
            reward_area: 1000,
            beneficiary_address: "miner1".to_string(),
        });
        let tx2 = Transaction::Coinbase(CoinbaseTx {
            reward_area: 2000,
            beneficiary_address: "miner2".to_string(),
        });
        let root = Block::calculate_merkle_root(&[tx1, tx2]);
        assert_eq!(root.len(), 64);
    }

    #[test]
    fn test_merkle_tree_odd() {
        let tx1 = Transaction::Coinbase(CoinbaseTx {
            reward_area: 1000,
            beneficiary_address: "miner1".to_string(),
        });
        let tx2 = Transaction::Coinbase(CoinbaseTx {
            reward_area: 2000,
            beneficiary_address: "miner2".to_string(),
        });
        let tx3 = Transaction::Coinbase(CoinbaseTx {
            reward_area: 3000,
            beneficiary_address: "miner3".to_string(),
        });
        let root = Block::calculate_merkle_root(&[tx1, tx2, tx3]);
        assert_eq!(root.len(), 64);
    }

    #[test]
    fn test_apply_block_updates_state() {
        let mut chain = Blockchain::new();
        let initial_count = chain.state.count();

        let genesis_hash = chain.state.utxo_set.keys().next().unwrap().clone();
        let genesis_tri = chain.state.utxo_set.get(&genesis_hash).unwrap().clone();
        let children = genesis_tri.subdivide();

        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();

        let mut tx = SubdivisionTx::new(genesis_hash, children.to_vec(), address.clone(), 0, 1);
        let message = tx.signable_message();
        let signature = keypair.sign(&message).unwrap();
        let public_key = keypair.public_key.serialize().to_vec();
        tx.sign(signature, public_key);

        let coinbase = CoinbaseTx {
            reward_area: 1000,
            beneficiary_address: address,
        };

        let transactions = vec![
            Transaction::Coinbase(coinbase),
            Transaction::Subdivision(tx),
        ];

        let last_block = chain.blocks.last().unwrap();
        let mut new_block = Block::new(
            last_block.height + 1,
            last_block.hash.clone(),
            chain.difficulty,
            transactions,
        );

        new_block.hash = new_block.calculate_hash();

        while !new_block.verify_proof_of_work() {
            new_block.nonce += 1;
            new_block.hash = new_block.calculate_hash();
        }

        chain.apply_block(new_block).unwrap();

        assert_eq!(chain.state.count(), initial_count + 2);
    }

    #[test]
    fn test_block_validation_success() {
        let mut chain = Blockchain::new();
        let genesis_hash = chain.state.utxo_set.keys().next().unwrap().clone();
        let genesis_tri = chain.state.utxo_set.get(&genesis_hash).unwrap().clone();
        let children = genesis_tri.subdivide();

        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();

        let mut tx = SubdivisionTx::new(genesis_hash, children.to_vec(), address.clone(), 0, 1);
        let message = tx.signable_message();
        let signature = keypair.sign(&message).unwrap();
        let public_key = keypair.public_key.serialize().to_vec();
        tx.sign(signature, public_key);

        let coinbase = CoinbaseTx {
            reward_area: 1000,
            beneficiary_address: address,
        };

        let transactions = vec![
            Transaction::Coinbase(coinbase),
            Transaction::Subdivision(tx),
        ];

        let last_block = chain.blocks.last().unwrap();
        let mut new_block = Block::new(
            last_block.height + 1,
            last_block.hash.clone(),
            chain.difficulty,
            transactions,
        );

        new_block.hash = new_block.calculate_hash();

        while !new_block.verify_proof_of_work() {
            new_block.nonce += 1;
            new_block.hash = new_block.calculate_hash();
        }

        assert!(chain.validate_block(&new_block).is_ok());
    }

    #[test]
    fn test_block_validation_failure_linkage() {
        let chain = Blockchain::new();
        let last_block = chain.blocks.last().unwrap();

        let mut bad_block = Block::new(
            last_block.height + 1,
            "wrong_hash".to_string(),
            chain.difficulty,
            vec![],
        );

        bad_block.hash = bad_block.calculate_hash();

        while !bad_block.verify_proof_of_work() {
            bad_block.nonce += 1;
            bad_block.hash = bad_block.calculate_hash();
        }

        assert!(chain.validate_block(&bad_block).is_err());
    }

    #[test]
    fn test_block_validation_failure_pow() {
        let chain = Blockchain::new();
        let last_block = chain.blocks.last().unwrap();

        let mut bad_block = Block::new(
            last_block.height + 1,
            last_block.hash.clone(),
            chain.difficulty,
            vec![],
        );

        bad_block.hash = "1234567890abcdef".to_string();

        assert!(chain.validate_block(&bad_block).is_err());
    }

    #[test]
    fn test_block_validation_double_spend_in_block() {
        let mut chain = Blockchain::new();
        let genesis_hash = chain.state.utxo_set.keys().next().unwrap().clone();
        let genesis_tri = chain.state.utxo_set.get(&genesis_hash).unwrap().clone();
        let children = genesis_tri.subdivide();

        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();

        let mut tx1 = SubdivisionTx::new(genesis_hash.clone(), children.to_vec(), address.clone(), 0, 1);
        let message1 = tx1.signable_message();
        let signature1 = keypair.sign(&message1).unwrap();
        let public_key1 = keypair.public_key.serialize().to_vec();
        tx1.sign(signature1, public_key1);

        let mut tx2 = SubdivisionTx::new(genesis_hash, children.to_vec(), address.clone(), 0, 2);
        let message2 = tx2.signable_message();
        let signature2 = keypair.sign(&message2).unwrap();
        let public_key2 = keypair.public_key.serialize().to_vec();
        tx2.sign(signature2, public_key2);

        let coinbase = CoinbaseTx {
            reward_area: 1000,
            beneficiary_address: address,
        };

        let transactions = vec![
            Transaction::Coinbase(coinbase),
            Transaction::Subdivision(tx1),
            Transaction::Subdivision(tx2),
        ];

        let last_block = chain.blocks.last().unwrap();
        let mut new_block = Block::new(
            last_block.height + 1,
            last_block.hash.clone(),
            chain.difficulty,
            transactions,
        );

        new_block.hash = new_block.calculate_hash();

        while !new_block.verify_proof_of_work() {
            new_block.nonce += 1;
            new_block.hash = new_block.calculate_hash();
        }

        assert!(chain.apply_block(new_block).is_err());
    }

    #[test]
    fn test_difficulty_adjustment_increase() {
        let mut chain = Blockchain::new();

        for i in 1..=10 {
            let block = Block {
                header: BlockHeader {
                    height: i,
                    previous_hash: chain.blocks.last().unwrap().hash.clone(),
                    timestamp: Utc::now().timestamp() + (i as i64 * 10),
                    difficulty: chain.difficulty,
                    nonce: 0,
                    merkle_root: String::new(),
                },
                hash: format!("{:064x}", i),
                height: i,
                previous_hash: chain.blocks.last().unwrap().hash.clone(),
                timestamp: Utc::now().timestamp() + (i as i64 * 10),
                difficulty: chain.difficulty,
                nonce: 0,
                merkle_root: String::new(),
                transactions: vec![],
            };

            chain.blocks.push(block);
            chain.adjust_difficulty();
        }

        assert!(chain.difficulty >= 2);
    }

    #[test]
    fn test_difficulty_adjustment_decrease() {
        let mut chain = Blockchain::new();

        for i in 1..=10 {
            let block = Block {
                header: BlockHeader {
                    height: i,
                    previous_hash: chain.blocks.last().unwrap().hash.clone(),
                    timestamp: Utc::now().timestamp() + (i as i64 * 200),
                    difficulty: chain.difficulty,
                    nonce: 0,
                    merkle_root: String::new(),
                },
                hash: format!("{:064x}", i),
                height: i,
                previous_hash: chain.blocks.last().unwrap().hash.clone(),
                timestamp: Utc::now().timestamp() + (i as i64 * 200),
                difficulty: chain.difficulty,
                nonce: 0,
                merkle_root: String::new(),
                transactions: vec![],
            };

            chain.blocks.push(block);
            chain.adjust_difficulty();
        }

        assert!(chain.difficulty <= 2);
    }

    #[test]
    fn test_difficulty_adjustment_no_change() {
        let mut chain = Blockchain::new();
        let initial_difficulty = chain.difficulty;

        for i in 1..=10 {
            let block = Block {
                header: BlockHeader {
                    height: i,
                    previous_hash: chain.blocks.last().unwrap().hash.clone(),
                    timestamp: Utc::now().timestamp() + (i as i64 * 60),
                    difficulty: chain.difficulty,
                    nonce: 0,
                    merkle_root: String::new(),
                },
                hash: format!("{:064x}", i),
                height: i,
                previous_hash: chain.blocks.last().unwrap().hash.clone(),
                timestamp: Utc::now().timestamp() + (i as i64 * 60),
                difficulty: chain.difficulty,
                nonce: 0,
                merkle_root: String::new(),
                transactions: vec![],
            };

            chain.blocks.push(block);
            chain.adjust_difficulty();
        }

        assert_eq!(chain.difficulty, initial_difficulty);
    }

    #[test]
    fn test_mempool_add_transaction() {
        let mut mempool = Mempool::new();
        let mut state = TriangleState::new();
        let genesis = genesis_triangle();
        let genesis_hash = genesis.hash();
        state.utxo_set.insert(genesis_hash.clone(), genesis.clone());
        let children = genesis.subdivide();
        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();
        let mut valid_tx = SubdivisionTx::new(genesis_hash.clone(), children.to_vec(), address, 0, 1);
        let message = valid_tx.signable_message();
        let signature = keypair.sign(&message).unwrap();
        let public_key = keypair.public_key.serialize().to_vec();
        valid_tx.sign(signature, public_key);
        let tx = Transaction::Subdivision(valid_tx);

        mempool.add_transaction(tx.clone()).unwrap();
        assert_eq!(mempool.len(), 1);
        assert!(!mempool.is_empty());
    }

    #[test]
    fn test_mempool_remove_transaction() {
        let mut mempool = Mempool::new();
        let mut state = TriangleState::new();
        let genesis = genesis_triangle();
        let genesis_hash = genesis.hash();
        state.utxo_set.insert(genesis_hash.clone(), genesis.clone());
        let children = genesis.subdivide();
        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();
        let mut valid_tx = SubdivisionTx::new(genesis_hash.clone(), children.to_vec(), address, 0, 1);
        let message = valid_tx.signable_message();
        let signature = keypair.sign(&message).unwrap();
        let public_key = keypair.public_key.serialize().to_vec();
        valid_tx.sign(signature, public_key);
        let tx = Transaction::Subdivision(valid_tx);
        let tx_hash = tx.hash();

        mempool.add_transaction(tx.clone()).unwrap();
        assert_eq!(mempool.len(), 1);

        let removed = mempool.remove_transaction(&tx_hash);
        assert!(removed.is_some());
        assert_eq!(mempool.len(), 0);
    }

    #[test]
    fn test_mempool_duplicate_transaction() {
        let mut mempool = Mempool::new();
        let mut state = TriangleState::new();
        let genesis = genesis_triangle();
        let genesis_hash = genesis.hash();
        state.utxo_set.insert(genesis_hash.clone(), genesis.clone());
        let children = genesis.subdivide();
        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();
        let mut valid_tx = SubdivisionTx::new(genesis_hash.clone(), children.to_vec(), address, 0, 1);
        let message = valid_tx.signable_message();
        let signature = keypair.sign(&message).unwrap();
        let public_key = keypair.public_key.serialize().to_vec();
        valid_tx.sign(signature, public_key);
        let tx = Transaction::Subdivision(valid_tx);

        mempool.add_transaction(tx.clone()).unwrap();
        let result = mempool.add_transaction(tx.clone());

        assert!(result.is_err());
        assert_eq!(mempool.len(), 1);
    }

    #[test]
    fn test_mempool_validate_and_prune() {
        let mut mempool = Mempool::new();
        let mut state = TriangleState::new();

        // Add genesis triangle to state
        let genesis = genesis_triangle();
        let genesis_hash = genesis.hash();
        state.utxo_set.insert(genesis_hash.clone(), genesis.clone());

        // Create valid subdivision transaction
        let children = genesis.subdivide();
        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();
        let mut valid_tx = SubdivisionTx::new(genesis_hash.clone(), children.to_vec(), address, 0, 1);
        let message = valid_tx.signable_message();
        let signature = keypair.sign(&message).unwrap();
        let public_key = keypair.public_key.serialize().to_vec();
        valid_tx.sign(signature, public_key);

        mempool.add_transaction(Transaction::Subdivision(valid_tx)).unwrap();

        // Create invalid subdivision (non-existent parent)
        let invalid_parent_hash = "nonexistent".to_string();
        let invalid_tx = SubdivisionTx::new(invalid_parent_hash, children.to_vec(), "addr".to_string(), 0, 1);
        mempool.add_transaction(Transaction::Subdivision(invalid_tx)).unwrap();

        // Should have 2 transactions
        assert_eq!(mempool.len(), 2);

        // Validate and prune - should remove 1 invalid transaction
        let removed = mempool.validate_and_prune(&state);
        assert_eq!(removed, 1);
        assert_eq!(mempool.len(), 1);
    }

    #[test]
    fn test_blockchain_with_mempool() {
        let mut chain = Blockchain::new();
        assert!(chain.mempool.is_empty());

        // Add a transaction to mempool
        let genesis = genesis_triangle();
        let genesis_hash = genesis.hash();
        let children = genesis.subdivide();
        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();
        let mut valid_tx = SubdivisionTx::new(genesis_hash.clone(), children.to_vec(), address, 0, 1);
        let message = valid_tx.signable_message();
        let signature = keypair.sign(&message).unwrap();
        let public_key = keypair.public_key.serialize().to_vec();
        valid_tx.sign(signature, public_key);
        let tx = Transaction::Subdivision(valid_tx);
        chain.mempool.add_transaction(tx.clone()).unwrap();
        assert_eq!(chain.mempool.len(), 1);

        // Create and apply a block with that transaction
        let last_block = chain.blocks.last().unwrap();
        let coinbase = CoinbaseTx {
            reward_area: 1000,
            beneficiary_address: "miner_address".to_string(),
        };
        let new_block = Block::new(
            last_block.height + 1,
            last_block.hash.clone(),
            chain.difficulty,
            vec![Transaction::Coinbase(coinbase), tx],
        );

        // Before applying, mempool has 1 transaction
        assert_eq!(chain.mempool.len(), 1);

        // Apply block (this should remove the transaction from mempool)
        let mut mined_block = new_block.clone();
        loop {
            mined_block.hash = mined_block.calculate_hash();
            if mined_block.verify_proof_of_work() {
                break;
            }
            mined_block.nonce += 1;
        }

        chain.apply_block(mined_block).unwrap();

        // After applying, mempool should be empty
        assert_eq!(chain.mempool.len(), 0);
    }
}
