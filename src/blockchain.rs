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

/// The blockchain itself
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Blockchain {
    pub blocks: Vec<Block>,
    pub state: TriangleState,
    pub difficulty: u64,
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

        Blockchain {
            blocks: vec![genesis_block],
            state,
            difficulty: 2,
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
                Transaction::Coinbase(_) => {},
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

        for tx in valid_block.transactions.iter() {
            match tx {
                Transaction::Subdivision(sub_tx) => {
                    self.state.apply_subdivision(sub_tx)?;
                },
                Transaction::Coinbase(cb_tx) => {
                    self.state.apply_coinbase(cb_tx);
                },
                Transaction::Transfer(tx) => {
                    let triangle = self.state.utxo_set.remove(&tx.input_hash)
                        .expect("Transfer input missing");
                    let new_hash = format!("{:x}", Sha256::digest(
                        format!("{}:{}", tx.input_hash, tx.new_owner).as_bytes()
                    ));
                    self.state.utxo_set.insert(new_hash, triangle);
                }
            }
        }

        self.blocks.push(valid_block);
        self.adjust_difficulty();

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
        let expected_time = TARGET_BLOCK_TIME_SECONDS * DIFFICULTY_ADJUSTMENT_WINDOW as i64;

        if time_taken < expected_time / 2 {
            self.difficulty += 1;
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

        let mut tx = SubdivisionTx::new(genesis_hash, children, address.clone(), 0, 1);
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

        let mut tx = SubdivisionTx::new(genesis_hash, children, address.clone(), 0, 1);
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

        let mut tx1 = SubdivisionTx::new(genesis_hash.clone(), children.clone(), address.clone(), 0, 1);
        let message1 = tx1.signable_message();
        let signature1 = keypair.sign(&message1).unwrap();
        let public_key1 = keypair.public_key.serialize().to_vec();
        tx1.sign(signature1, public_key1);

        let mut tx2 = SubdivisionTx::new(genesis_hash, children, address.clone(), 0, 2);
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
}
