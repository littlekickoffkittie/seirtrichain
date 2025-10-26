//! Transaction types and validation for siertrichain.
//! Handles subdivision transactions and coinbase rewards.

use serde::{Serialize, Deserialize};
use sha2::{Digest, Sha256};
use crate::geometry::Triangle;
use crate::blockchain::TriangleState;
use crate::error::ChainError;
use crate::crypto::{verify_signature, Address};

/// Transaction types in siertrichain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Transaction {
    Subdivision(SubdivisionTx),
    Coinbase(CoinbaseTx),
}

impl Transaction {
    /// Calculate the hash of this transaction
    pub fn hash(&self) -> String {
        let tx_data = match self {
            Transaction::Subdivision(tx) => format!(
                "{}{}{}{}{}",
                tx.parent_hash,
                tx.children.iter().map(|c| c.hash()).collect::<Vec<_>>().join(""),
                tx.owner_address,
                tx.fee,
                tx.nonce,
            ),
            Transaction::Coinbase(tx) => format!(
                "coinbase{}{}",
                tx.reward_area,
                tx.beneficiary_address,
            ),
        };
        
        let mut hasher = Sha256::new();
        hasher.update(tx_data.as_bytes());
        format!("{:x}", hasher.finalize())
    }
    
    /// Validate this transaction against the current UTXO state
    pub fn validate(&self, state: &TriangleState) -> Result<(), ChainError> {
        match self {
            Transaction::Subdivision(tx) => tx.validate(state),
            Transaction::Coinbase(tx) => tx.validate(),
        }
    }
}

/// A transaction that subdivides a parent triangle into three child triangles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubdivisionTx {
    pub parent_hash: String,
    pub children: [Triangle; 3],
    pub owner_address: Address,
    pub signature: Vec<u8>,
    pub public_key: Vec<u8>,
    pub fee: u64,
    pub nonce: u64,
}

impl SubdivisionTx {
    /// Create a new subdivision transaction
    pub fn new(
        parent_hash: String,
        children: [Triangle; 3],
        owner_address: Address,
        fee: u64,
        nonce: u64,
    ) -> Self {
        SubdivisionTx {
            parent_hash,
            children,
            owner_address,
            signature: Vec::new(),
            public_key: Vec::new(),
            fee,
            nonce,
        }
    }
    
    /// Get the signable message for this transaction
    pub fn signable_message(&self) -> Vec<u8> {
        let msg = format!(
            "{}{}{}{}{}",
            self.parent_hash,
            self.children.iter().map(|c| c.hash()).collect::<Vec<_>>().join(""),
            self.owner_address,
            self.fee,
            self.nonce,
        );
        msg.into_bytes()
    }
    
    /// Sign this transaction with a keypair
    pub fn sign(&mut self, signature: Vec<u8>, public_key: Vec<u8>) {
        self.signature = signature;
        self.public_key = public_key;
    }
    
    /// Validate this subdivision transaction
    pub fn validate(&self, state: &TriangleState) -> Result<(), ChainError> {
        // 1. Check that parent triangle exists in UTXO set
        let parent = state.get_triangle(&self.parent_hash)?;
        
        // 2. Verify the subdivision is geometrically correct
        let expected_children = parent.subdivide();
        for (i, child) in self.children.iter().enumerate() {
            if child.hash() != expected_children[i].hash() {
                return Err(ChainError::InvalidTransaction(
                    format!("Child triangle {} does not match expected subdivision", i)
                ));
            }
        }
        
        // 3. Verify cryptographic signature
        if self.signature.is_empty() || self.public_key.is_empty() {
            return Err(ChainError::InvalidTransaction(
                "Transaction must be signed".to_string()
            ));
        }
        
        let message = self.signable_message();
        let is_valid = verify_signature(&self.public_key, &message, &self.signature)?;
        
        if !is_valid {
            return Err(ChainError::InvalidTransaction(
                "Invalid signature".to_string()
            ));
        }
        
        Ok(())
    }
}

/// A coinbase transaction that rewards the miner
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinbaseTx {
    pub reward_area: u64,
    pub beneficiary_address: Address,
}

impl CoinbaseTx {
    /// Validate this coinbase transaction
    pub fn validate(&self) -> Result<(), ChainError> {
        // Basic validation - reward must be positive
        if self.reward_area == 0 {
            return Err(ChainError::InvalidTransaction(
                "Coinbase reward must be positive".to_string()
            ));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Triangle;
    use crate::crypto::KeyPair;
    
    fn create_test_state_with_genesis() -> TriangleState {
        let mut state = TriangleState::new();
        state.init_genesis();
        state
    }
    
    #[test]
    fn test_tx_validation_success() {
        let state = create_test_state_with_genesis();
        let genesis_triangle = Triangle::genesis();
        let children = genesis_triangle.subdivide();
        
        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();
        
        let mut tx = SubdivisionTx::new(
            genesis_triangle.hash(),
            children,
            address,
            100,
            1,
        );
        
        let message = tx.signable_message();
        let signature = keypair.sign(&message).unwrap();
        let public_key = keypair.public_key.serialize().to_vec();
        tx.sign(signature, public_key);
        
        let result = tx.validate(&state);
        if let Err(e) = &result {
            eprintln!("DEBUG: Validation error: {:?}", e);
        }
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_tx_validation_double_spend_check() {
        let mut state = create_test_state_with_genesis();
        let genesis_triangle = Triangle::genesis();
        let children = genesis_triangle.subdivide();
        
        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();
        
        let mut tx1 = SubdivisionTx::new(
            genesis_triangle.hash(),
            children.clone(),
            address.clone(),
            100,
            1,
        );
        
        let message = tx1.signable_message();
        let signature = keypair.sign(&message).unwrap();
        let public_key = keypair.public_key.serialize().to_vec();
        tx1.sign(signature, public_key);
        
        assert!(tx1.validate(&state).is_ok());
        
        state.utxo_set.remove(&genesis_triangle.hash());
        
        let mut tx2 = SubdivisionTx::new(
            genesis_triangle.hash(),
            children,
            address,
            100,
            2,
        );
        
        let message2 = tx2.signable_message();
        let signature2 = keypair.sign(&message2).unwrap();
        let public_key2 = keypair.public_key.serialize().to_vec();
        tx2.sign(signature2, public_key2);
        
        match tx2.validate(&state) {
            Err(ChainError::TriangleNotFound(_)) => assert!(true),
            _ => panic!("Expected TriangleNotFound error"),
        }
    }
    
    #[test]
    fn test_tx_validation_area_conservation_failure() {
        let state = create_test_state_with_genesis();
        let genesis_triangle = Triangle::genesis();
        let mut children = genesis_triangle.subdivide();
        
        children[0].a.x += 0.1;
        
        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();
        
        let mut tx = SubdivisionTx::new(
            genesis_triangle.hash(),
            children,
            address,
            100,
            1,
        );
        
        let message = tx.signable_message();
        let signature = keypair.sign(&message).unwrap();
        let public_key = keypair.public_key.serialize().to_vec();
        tx.sign(signature, public_key);
        
        match tx.validate(&state) {
            Err(ChainError::InvalidTransaction(_)) => assert!(true),
            _ => panic!("Expected InvalidTransaction error"),
        }
    }
    
    #[test]
    fn test_unsigned_transaction_fails() {
        let state = create_test_state_with_genesis();
        let genesis_triangle = Triangle::genesis();
        let children = genesis_triangle.subdivide();
        
        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();
        
        let tx = SubdivisionTx::new(
            genesis_triangle.hash(),
            children,
            address,
            100,
            1,
        );
        
        match tx.validate(&state) {
            Err(ChainError::InvalidTransaction(msg)) if msg.contains("must be signed") => assert!(true),
            _ => panic!("Expected 'must be signed' error"),
        }
    }
    
    #[test]
    fn test_invalid_signature_fails() {
        let state = create_test_state_with_genesis();
        let genesis_triangle = Triangle::genesis();
        let children = genesis_triangle.subdivide();
        
        let keypair1 = KeyPair::generate().unwrap();
        let keypair2 = KeyPair::generate().unwrap();
        let address = keypair1.address();
        
        let mut tx = SubdivisionTx::new(
            genesis_triangle.hash(),
            children,
            address,
            100,
            1,
        );
        
        let message = tx.signable_message();
        let signature = keypair1.sign(&message).unwrap();
        let wrong_public_key = keypair2.public_key.serialize().to_vec();
        tx.sign(signature, wrong_public_key);
        
        match tx.validate(&state) {
            Err(ChainError::InvalidTransaction(msg)) if msg.contains("Invalid signature") => assert!(true),
            _ => panic!("Expected 'Invalid signature' error"),
        }
    }
}
