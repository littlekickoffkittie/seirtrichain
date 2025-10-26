#!/bin/bash

# This will update the test section of blockchain.rs with real crypto

# Find the line number where tests module starts
START_LINE=$(grep -n "^#\[cfg(test)\]" ~/siertrichain/src/blockchain.rs | tail -1 | cut -d: -f1)

# Keep everything before the test module
head -n $((START_LINE - 1)) ~/siertrichain/src/blockchain.rs > ~/siertrichain/src/blockchain_new.rs

# Append the new test module with real crypto
cat << 'TESTEOF' >> ~/siertrichain/src/blockchain_new.rs

// ---------------------------------------------------
-------------------------
// Tests
// ---------------------------------------------------
-------------------------

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
        let children = t.subdivide();
        
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
TESTEOF

# Replace the old file
mv ~/siertrichain/src/blockchain_new.rs ~/siertrichain/src/blockchain.rs

echo "✅ Blockchain tests updated with real cryptography!"
