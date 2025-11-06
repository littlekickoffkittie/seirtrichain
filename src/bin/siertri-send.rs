//! Send triangles to another address

use siertrichain::persistence::Database;
use siertrichain::transaction::{Transaction, TransferTx, CoinbaseTx};
use siertrichain::crypto::KeyPair;
use siertrichain::miner::mine_block;
use secp256k1::SecretKey;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        println!("Usage: siertri-send <to_address> <triangle_hash> [memo]");
        println!("\nExamples:");
        println!("  siertri-send abc123... def456...");
        println!("  siertri-send abc123... def456... \"Payment for services\"");
        std::process::exit(1);
    }

    let to_address = &args[1];
    let triangle_hash = &args[2];
    let memo = if args.len() > 3 {
        Some(args[3..].join(" "))
    } else {
        None
    };

    println!("💸 Sending triangle...\n");

    let home = std::env::var("HOME")?;
    let wallet_file = format!("{}/.siertrichain/wallet.json", home);

    let wallet_content = std::fs::read_to_string(&wallet_file)?;
    let wallet_data: serde_json::Value = serde_json::from_str(&wallet_content)?;

    let from_address = wallet_data["address"].as_str()
        .ok_or("Wallet address not found")?
        .to_string();
    let secret_hex = wallet_data["secret_key"].as_str()
        .ok_or("Secret key not found")?;
    let secret_bytes = hex::decode(secret_hex)?;
    let secret_key = SecretKey::from_slice(&secret_bytes)?;
    let keypair = KeyPair::from_secret_key(secret_key);

    let db = Database::open("siertrichain.db")?;
    let mut chain = db.load_blockchain()?;

    let full_hash = *chain.state.utxo_set.keys()
        .find(|h| hex::encode(h).starts_with(triangle_hash))
        .ok_or_else(|| format!("Triangle with hash prefix {} not found", triangle_hash))?;

    let triangle = chain.state.utxo_set.get(&full_hash)
        .ok_or("Triangle not found in UTXO set")?
        .clone();

    let full_hash_hex = hex::encode(full_hash);
    let full_hash_prefix = &full_hash_hex[..16];
    let from_prefix = if from_address.len() >= 16 { &from_address[..16] } else { &from_address };
    let to_prefix = if to_address.len() >= 16 { &to_address[..16] } else { to_address };

    println!("🔺 Triangle: {}", full_hash_prefix);
    println!("   Area: {:.6}", triangle.area());
    println!("   From: {}...", from_prefix);
    println!("   To: {}...", to_prefix);
    if let Some(ref m) = memo {
        println!("   📝 Memo: {}", m);
    }
    println!();

    let mut tx = TransferTx::new(full_hash, to_address.to_string(), from_address.clone(), 0, chain.blocks.len() as u64);

    // Add memo if provided
    if let Some(m) = memo {
        tx = tx.with_memo(m)?;
    }

    let message = tx.signable_message();
    let signature = keypair.sign(&message)?;
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

    let last_block = chain.blocks.last()
        .ok_or("Blockchain is empty")?;
    let mut new_block = siertrichain::blockchain::Block::new(
        last_block.header.height + 1,
        last_block.hash,
        chain.difficulty,
        transactions,
    );

    new_block = mine_block(new_block)?;

    let new_hash_hex = hex::encode(new_block.hash);
    let new_hash_prefix = &new_hash_hex[..16];
    println!("✅ Block mined! Hash: {}", new_hash_prefix);

    chain.apply_block(new_block.clone())?;

    db.save_block(&new_block)?;
    db.save_utxo_set(&chain.state)?;

    println!("\n🎉 Transfer complete!");
    println!("   Block: {}", chain.blocks.len() - 1);

    Ok(())
}
