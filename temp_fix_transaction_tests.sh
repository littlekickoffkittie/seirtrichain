#!/bin/bash

# Replace the test functions in transaction.rs with fixed versions
cat << 'TXEOF' > ~/siertrichain/src/transaction_tests_fixed.rs

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
        let children_vec = genesis_triangle.subdivide();
        let children: [Triangle; 3] = [
            children_vec[0].clone(),
            children_vec[1].clone(),
            children_vec[2].clone(),
        ];
        
        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();
        
        let mut tx = SubdivisionTx::new(
            genesis_triangle.hash(),
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
        
        assert!(tx.validate(&state).is_ok());
    }
    
    #[test]
    fn test_tx_validation_double_spend_check() {
        let mut state = create_test_state_with_genesis();
        let genesis_triangle = Triangle::genesis();
        let children_vec = genesis_triangle.subdivide();
        let children: [Triangle; 3] = [
            children_vec[0].clone(),
            children_vec[1].clone(),
            children_vec[2].clone(),
        ];
        
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
        
        // First transaction should succeed
        assert!(tx1.validate(&state).is_ok());
        
        // Remove parent from state (simulating it was spent)
        state.utxo_set.remove(&genesis_triangle.hash());
        
        // Second identical transaction should fail (double spend)
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
        let children_vec = genesis_triangle.subdivide();
        let mut children: [Triangle; 3] = [
            children_vec[0].clone(),
            children_vec[1].clone(),
            children_vec[2].clone(),
        ];
        
        // Tamper with one child's area by modifying point
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
        let children_vec = genesis_triangle.subdivide();
        let children: [Triangle; 3] = [
            children_vec[0].clone(),
            children_vec[1].clone(),
            children_vec[2].clone(),
        ];
        
        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();
        
        let tx = SubdivisionTx::new(
            genesis_triangle.hash(),
            children,
            address,
            100,
            1,
        );
        
        // Don't sign - should fail
        match tx.validate(&state) {
            Err(ChainError::InvalidTransaction(msg)) if msg.contains("must be signed") => assert!(true),
            _ => panic!("Expected 'must be signed' error"),
        }
    }
    
    #[test]
    fn test_invalid_signature_fails() {
        let state = create_test_state_with_genesis();
        let genesis_triangle = Triangle::genesis();
        let children_vec = genesis_triangle.subdivide();
        let children: [Triangle; 3] = [
            children_vec[0].clone(),
            children_vec[1].clone(),
            children_vec[2].clone(),
        ];
        
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
        
        // Sign with keypair1 but use keypair2's public key
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
TXEOF

# Now rebuild the transaction.rs file with fixed tests
head -n $(grep -n "^#\[cfg(test)\]" ~/siertrichain/src/transaction.rs | cut -d: -f1) ~/siertrichain/src/transaction.rs | head -n -1 > ~/siertrichain/src/transaction_new.rs
cat ~/siertrichain/src/transaction_tests_fixed.rs >> ~/siertrichain/src/transaction_new.rs
mv ~/siertrichain/src/transaction_new.rs ~/siertrichain/src/transaction.rs
rm ~/siertrichain/src/transaction_tests_fixed.rs
