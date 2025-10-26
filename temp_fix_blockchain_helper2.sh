#!/bin/bash

# Find and replace the create_dummy_transaction function
START=$(grep -n "fn create_dummy_transaction()" ~/siertrichain/src/blockchain.rs | cut -d: -f1)
END=$(awk "NR>$START && /^    }$/ {print NR; exit}" ~/siertrichain/src/blockchain.rs)

# Get everything before the function
head -n $((START - 1)) ~/siertrichain/src/blockchain.rs > ~/siertrichain/src/blockchain_temp.rs

# Add the fixed function
cat << 'FUNCEOF' >> ~/siertrichain/src/blockchain_temp.rs
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
FUNCEOF

# Add everything after the function
tail -n +$((END + 1)) ~/siertrichain/src/blockchain.rs >> ~/siertrichain/src/blockchain_temp.rs

mv ~/siertrichain/src/blockchain_temp.rs ~/siertrichain/src/blockchain.rs
