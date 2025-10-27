//! Mine a new block by subdividing a triangle

use siertrichain::persistence::Database;
use siertrichain::transaction::{Transaction, SubdivisionTx, CoinbaseTx};
use siertrichain::crypto::KeyPair;
use siertrichain::miner::mine_block;
use secp256k1::SecretKey;

fn main() {
    println!("⛏️  Mining Block...\n");
    
    let db = Database::open("siertrichain.db").expect("Failed to open database");
    let mut chain = db.load_blockchain().expect("Failed to load blockchain");
    
    println!("📊 Current height: {}", chain.blocks.last().unwrap().height);
    
    let wallet_file = std::env::var("HOME").unwrap() + "/.siertrichain/wallet.json";
    let wallet_data: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&wallet_file).expect("No wallet found")
    ).expect("Failed to parse wallet");
    
    let address = wallet_data["address"].as_str().unwrap().to_string();
    let secret_hex = wallet_data["secret_key"].as_str().unwrap();
    let secret_bytes = hex::decode(secret_hex).expect("Invalid secret key");
    let secret_key = SecretKey::from_slice(&secret_bytes).expect("Invalid key");
    let keypair = KeyPair::from_secret_key(secret_key);
    
    let parent_hash = chain.state.utxo_set.keys().next().unwrap().clone();
    let parent_triangle = chain.state.utxo_set.get(&parent_hash).unwrap().clone();
    
    println!("🔺 Subdividing triangle {}...", &parent_hash[..16]);
    let children = parent_triangle.subdivide();
    
    let mut tx = SubdivisionTx::new(parent_hash, children.to_vec(), address.clone(), 0, chain.blocks.len() as u64);
    let message = tx.signable_message();
    let signature = keypair.sign(&message).unwrap();
    let public_key = keypair.public_key.serialize().to_vec();
    tx.sign(signature, public_key);
    
    let coinbase = CoinbaseTx { reward_area: 1000, beneficiary_address: address };
    
    let transactions = vec![
        Transaction::Coinbase(coinbase),
        Transaction::Subdivision(tx),
    ];
    
    println!("⛏️  Mining block (difficulty {})...", chain.difficulty);
    
    let last_block = chain.blocks.last().unwrap();
    let mut new_block = siertrichain::blockchain::Block::new(
        last_block.height + 1,
        last_block.hash.clone(),
        chain.difficulty,
        transactions,
    );
    
    new_block = mine_block(new_block).expect("Mining failed");
    
    println!("✅ Block mined! Hash: {}", &new_block.hash[..16]);
    
    chain.apply_block(new_block.clone()).expect("Failed to apply block");
    
    db.save_block(&new_block).expect("Failed to save");
    db.save_utxo_set(&chain.state).expect("Failed to save UTXO");
    
    println!("\n🎉 Block {} mined successfully!", chain.blocks.len() - 1);
    println!("   UTXOs: {}", chain.state.count());
}
