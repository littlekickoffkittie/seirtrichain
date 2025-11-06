//! Check wallet balance

use siertrichain::persistence::Database;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let home = std::env::var("HOME")?;
    let wallet_file = format!("{}/.siertrichain/wallet.json", home);

    let wallet_content = std::fs::read_to_string(&wallet_file)
        .map_err(|e| format!("No wallet found at {}: {}", wallet_file, e))?;

    let wallet_data: serde_json::Value = serde_json::from_str(&wallet_content)
        .map_err(|e| format!("Failed to parse wallet: {}", e))?;

    let my_address = wallet_data["address"].as_str()
        .ok_or("Wallet address not found in wallet file")?;

    let db = Database::open("siertrichain.db")
        .map_err(|e| format!("Failed to open database: {}", e))?;
    let chain = db.load_blockchain()
        .map_err(|e| format!("Failed to load blockchain: {}", e))?;

    println!("💰 Wallet Balance\n");
    println!("📍 Address: {}", my_address);

    let height = chain.blocks.last()
        .map(|b| b.header.height)
        .unwrap_or(0);
    println!("📊 Chain Height: {}", height);
    println!("\n🔺 Your Triangles:");

    let mut my_triangles = 0;
    let mut total_area = 0.0;

    for (hash, triangle) in &chain.state.utxo_set {
        my_triangles += 1;
        total_area += triangle.area();
        let hash_hex = hex::encode(hash);
        let hash_prefix = &hash_hex[..16];
        println!("  • {} (area: {:.6})", hash_prefix, triangle.area());
    }

    println!("\n📈 Total: {} triangles", my_triangles);
    println!("📐 Combined area: {:.6}", total_area);

    Ok(())
}
