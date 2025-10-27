//! Send triangles to another address

use siertrichain::persistence::Database;
use siertrichain::transaction::{Transaction, TransferTx, CoinbaseTx};
use siertrichain::crypto::KeyPair;
use siertrichain::miner::mine_block;
use secp256k1::SecretKey;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 3 {
        println!("Usage: siertri-send <to_address> <triangle_hash>");
        return;
    }
    
    let to_address = &args[1];
    let triangle_hash = &args[2];
    
    println!("💸 Sending triangle...\n");
    
    let wallet_file = std::env::var("HOME").unwrap() + "/.siertrichain/wallet.json";
    let wallet_data: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&wallet_file).expect("No wallet found")
    ).expect("Failed to parse wallet");
    
    let from_address = wallet_data["address"].as_str().unwrap().to_string();
    let secret_hex = wallet_data["secret_key"].as_str().unwrap();
    let secret_bytes = hex::decode(secret_hex).expect("Invalid secret key");
    let secret_key = SecretKey::from_slice(&secret_bytes).expect("Invalid key");
    let keypair = KeyPair::from_secret_key(secret_key);
    
    let db = Database::open("siertrichain.db").expect("Failed to open database");
    let mut chain = db.load_blockchain().expect("Failed to load blockchain");
    
    let full_hash = chain.state.utxo_set.keys()
        .find(|h| h.starts_with(triangle_hash))
        .expect("Triangle not found")
        .clone();
    
    let triangle = chain.state.utxo_set.get(&full_hash).unwrap().clone();
    
    println!("🔺 Triangle: {}", &full_hash[..16]);
    println!("   Area: {:.6}", triangle.area());
    println!("   From: {}...", &from_address[..16]);
    println!("   To: {}...\n", &to_address[..16]);
    
    let mut tx = TransferTx::new(full_hash.clone(), to_address.to_string(), from_address.clone(), 0, chain.blocks.len() as u64);
    let message = tx.signable_message();
    let signature = keypair.sign(&message).unwrap();
    let public_key = keypair.public_key.serialize().to_vec();
    tx.sign(signature, public_key);
    
    let coinbase = CoinbaseTx { 
        reward_area: 1000, 
        beneficiary_address: from_address.clone() 
    };
    
    let transactions = vec![
        Transaction::Coinbase(coinbase),
        Transaction::Transfer(tx),
    ];
    
    println!("⛏️  Mining transaction into block...");
    
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
    
    println!("\n🎉 Transfer complete!");
    println!("   Block: {}", chain.blocks.len() - 1);
}
