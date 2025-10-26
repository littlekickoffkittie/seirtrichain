//! Miner CLI for siertrichain

use siertrichain::blockchain::Blockchain;
use siertrichain::persistence::Database;

fn main() {
    println!("⛏️  siertri-miner v0.1.0\n");
    
    let db = Database::open("siertrichain.db").expect("Failed to open database");
    let height = db.get_block_height().expect("Failed to get block height");
    
    if height > 0 {
        println!("📊 Blockchain already initialized (height: {})", height);
        return;
    }
    
    println!("🔺 Initializing genesis block...\n");
    let chain = Blockchain::new();
    
    println!("\n💾 Saving to database...");
    db.save_block(&chain.blocks[0]).expect("Failed to save genesis");
    db.save_utxo_set(&chain.state).expect("Failed to save UTXO");
    db.save_difficulty(chain.difficulty).expect("Failed to save difficulty");
    
    println!("✅ Genesis block mined and saved!");
    println!("\n📈 Blockchain Status:");
    println!("   Height: 0");
    println!("   Difficulty: {}", chain.difficulty);
    println!("   UTXO Count: {}", chain.state.count());
    println!("\n🎉 The genesis triangle is now in circulation!");
}
