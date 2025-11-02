//! Transaction types for siertrichain

use sha2::{Digest, Sha256};
use crate::blockchain::TriangleState;
use crate::geometry::Triangle;
use crate::error::ChainError;

pub type Address = String;

/// A transaction that can occur in a block
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Transaction {
    Transfer(TransferTx),
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
            Transaction::Transfer(tx) => format!(
                "transfer{}{}{}{}{}",
                tx.input_hash, tx.new_owner, tx.sender, tx.fee, tx.nonce
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
            Transaction::Transfer(tx) => tx.validate(),
        }
    }
}

/// Subdivision transaction: splits one parent triangle into three children
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SubdivisionTx {
    pub parent_hash: String,
    pub children: Vec<Triangle>,
    pub owner_address: Address,
    pub fee: u64,
    pub nonce: u64,
    pub signature: Option<Vec<u8>>,
    pub public_key: Option<Vec<u8>>,
}

impl SubdivisionTx {
    pub fn new(
        parent_hash: String,
        children: Vec<Triangle>,
        owner_address: Address,
        fee: u64,
        nonce: u64,
    ) -> Self {
        SubdivisionTx {
            parent_hash,
            children,
            owner_address,
            fee,
            nonce,
            signature: None,
            public_key: None,
        }
    }

    pub fn signable_message(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.extend_from_slice(self.parent_hash.as_bytes());
        for child in &self.children {
            message.extend_from_slice(child.hash().as_bytes());
        }
        message.extend_from_slice(self.owner_address.as_bytes());
        message.extend_from_slice(&self.fee.to_le_bytes());
        message.extend_from_slice(&self.nonce.to_le_bytes());
        message
    }

    pub fn sign(&mut self, signature: Vec<u8>, public_key: Vec<u8>) {
        self.signature = Some(signature);
        self.public_key = Some(public_key);
    }

    pub fn validate(&self, state: &TriangleState) -> Result<(), ChainError> {
        if !state.utxo_set.contains_key(&self.parent_hash) {
            return Err(ChainError::TriangleNotFound(format!(
                "Parent triangle {} not found in UTXO set",
                self.parent_hash
            )));
        }

        let parent = state.utxo_set.get(&self.parent_hash).unwrap();
        let expected_children = parent.subdivide();

        if self.children.len() != 3 {
            return Err(ChainError::InvalidTransaction(
                "Subdivision must produce exactly 3 children".to_string(),
            ));
        }

        for (i, child) in self.children.iter().enumerate() {
            if (child.a.x - expected_children[i].a.x).abs() > 1e-10 ||
               (child.a.y - expected_children[i].a.y).abs() > 1e-10 ||
               (child.b.x - expected_children[i].b.x).abs() > 1e-10 ||
               (child.b.y - expected_children[i].b.y).abs() > 1e-10 ||
               (child.c.x - expected_children[i].c.x).abs() > 1e-10 ||
               (child.c.y - expected_children[i].c.y).abs() > 1e-10 {
                return Err(ChainError::InvalidTransaction(format!(
                    "Child {} does not match expected subdivision",
                    i
                )));
            }
        }

        if self.signature.is_none() || self.public_key.is_none() {
            return Err(ChainError::InvalidTransaction(
                "Transaction not signed".to_string(),
            ));
        }

        let message = self.signable_message();
        let is_valid = crate::crypto::verify_signature(
            self.public_key.as_ref().unwrap(),
            &message,
            self.signature.as_ref().unwrap(),
        )?;

        if !is_valid {
            return Err(ChainError::InvalidTransaction(
                "Invalid signature".to_string(),
            ));
        }

        Ok(())
    }
}

/// Coinbase transaction: miner reward
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoinbaseTx {
    pub reward_area: u64,
    pub beneficiary_address: Address,
}

impl CoinbaseTx {
    pub fn validate(&self) -> Result<(), ChainError> {
        Ok(())
    }
}

/// Transfer transaction - moves ownership of a triangle
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransferTx {
    pub input_hash: String,
    pub new_owner: Address,
    pub sender: Address,
    pub fee: u64,
    pub nonce: u64,
    pub signature: Option<Vec<u8>>,
    pub public_key: Option<Vec<u8>>,
}

impl TransferTx {
    pub fn new(input_hash: String, new_owner: Address, sender: Address, fee: u64, nonce: u64) -> Self {
        TransferTx {
            input_hash,
            new_owner,
            sender,
            fee,
            nonce,
            signature: None,
            public_key: None,
        }
    }
    
    pub fn signable_message(&self) -> Vec<u8> {
        format!("TRANSFER:{}:{}:{}:{}:{}", 
            self.input_hash, self.new_owner, self.sender, self.fee, self.nonce)
            .into_bytes()
    }
    
    pub fn sign(&mut self, signature: Vec<u8>, public_key: Vec<u8>) {
        self.signature = Some(signature);
        self.public_key = Some(public_key);
    }
    
    pub fn validate(&self) -> Result<(), ChainError> {
        if self.signature.is_none() || self.public_key.is_none() {
            return Err(ChainError::InvalidTransaction("Transfer not signed".to_string()));
        }
        
        let message = self.signable_message();
        let is_valid = crate::crypto::verify_signature(
            self.public_key.as_ref().unwrap(),
            &message,
            self.signature.as_ref().unwrap(),
        )?;
        
        if !is_valid {
            return Err(ChainError::InvalidTransaction("Invalid signature".to_string()));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::TriangleState;
    use crate::crypto::KeyPair;
    use crate::geometry::{Point, Triangle};

    #[test]
    fn test_tx_validation_success() {
        let mut state = TriangleState::new();
        let parent = Triangle::new(
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 0.0 },
            Point { x: 0.5, y: 0.866 },
            None,
        );
        let parent_hash = parent.hash();
        state.utxo_set.insert(parent_hash.clone(), parent.clone());

        let children = parent.subdivide();
        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();

        let mut tx = SubdivisionTx::new(parent_hash, children.to_vec(), address, 0, 1);
        let message = tx.signable_message();
        let signature = keypair.sign(&message).unwrap();
        let public_key = keypair.public_key.serialize().to_vec();
        tx.sign(signature, public_key);

        assert!(tx.validate(&state).is_ok());
    }

    #[test]
    fn test_unsigned_transaction_fails() {
        let mut state = TriangleState::new();
        let parent = Triangle::new(
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 0.0 },
            Point { x: 0.5, y: 0.866 },
            None,
        );
        let parent_hash = parent.hash();
        state.utxo_set.insert(parent_hash.clone(), parent.clone());

        let children = parent.subdivide();
        let address = "test_address".to_string();

        let tx = SubdivisionTx::new(parent_hash, children.to_vec(), address, 0, 1);
        assert!(tx.validate(&state).is_err());
    }

    #[test]
    fn test_invalid_signature_fails() {
        let mut state = TriangleState::new();
        let parent = Triangle::new(
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 0.0 },
            Point { x: 0.5, y: 0.866 },
            None,
        );
        let parent_hash = parent.hash();
        state.utxo_set.insert(parent_hash.clone(), parent.clone());

        let children = parent.subdivide();
        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();

        let mut tx = SubdivisionTx::new(parent_hash, children.to_vec(), address, 0, 1);
        let fake_signature = vec![0u8; 64];
        let public_key = keypair.public_key.serialize().to_vec();
        tx.sign(fake_signature, public_key);

        assert!(tx.validate(&state).is_err());
    }

    #[test]
    fn test_tx_validation_area_conservation_failure() {
        let mut state = TriangleState::new();
        let parent = Triangle::new(
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 0.0 },
            Point { x: 0.5, y: 0.866 },
            None,
        );
        let parent_hash = parent.hash();
        state.utxo_set.insert(parent_hash.clone(), parent);

        let bad_child = Triangle::new(
            Point { x: 0.0, y: 0.0 },
            Point { x: 2.0, y: 0.0 },
            Point { x: 1.0, y: 1.732 },
            None,
        );
        let children = vec![bad_child.clone(), bad_child.clone(), bad_child];

        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();

        let tx = SubdivisionTx::new(parent_hash, children, address, 0, 1);
        assert!(tx.validate(&state).is_err());
    }

    #[test]
    fn test_tx_validation_double_spend_check() {
        let state = TriangleState::new();

        let parent = Triangle::new(
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 0.0 },
            Point { x: 0.5, y: 0.866 },
            None,
        );
        let parent_hash = parent.hash();
        let children = parent.subdivide();

        let address = "test_address".to_string();
        let tx = SubdivisionTx::new(parent_hash, children.to_vec(), address, 0, 1);

        assert!(tx.validate(&state).is_err());
    }
}
