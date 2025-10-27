#!/usr/bin/env python3
import re

# Read the file
with open('src/blockchain.rs', 'r') as f:
    content = f.read()

# Fix 1: Add Transfer to validate_block function
# Find the match statement around line 305
validation_pattern = r'(for tx in valid_block\.transactions\.iter\(\) \{[\s\S]*?match tx \{[\s\S]*?Transaction::Subdivision\(tx\) => \{[\s\S]*?\},)'
validation_replacement = r'''\1
                Transaction::Transfer(tx) => {
                    if !self.state.utxo_set.contains_key(&tx.input_hash) {
                        return Err(ChainError::InvalidTransaction(
                            format!("Transfer input {} not in UTXO set", tx.input_hash)
                        ));
                    }
                    tx.validate()?;
                },'''

content = re.sub(validation_pattern, validation_replacement, content)

# Fix 2: Add Transfer to apply_block function  
# Find the match statement around line 323
apply_pattern = r'(for tx in valid_block\.transactions\.iter\(\) \{[\s\S]*?match tx \{[\s\S]*?Transaction::Coinbase\(cb_tx\) => \{[\s\S]*?self\.state\.apply_coinbase\(cb_tx\);[\s\S]*?\})'
apply_replacement = r'''\1
                Transaction::Transfer(tx) => {
                    let triangle = self.state.utxo_set.remove(&tx.input_hash)
                        .expect("Transfer input missing");
                    let new_hash = format!("{:x}", Sha256::digest(
                        format!("{}:{}", tx.input_hash, tx.new_owner).as_bytes()
                    ));
                    self.state.utxo_set.insert(new_hash, triangle);
                }'''

content = re.sub(apply_pattern, apply_replacement, content)

# Write back
with open('src/blockchain.rs', 'w') as f:
    f.write(content)

print("✅ blockchain.rs updated with Transfer handling")
