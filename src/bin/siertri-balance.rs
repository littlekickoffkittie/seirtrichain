//! Check wallet balance

use siertrichain::persistence::Database;

fn main() {
    let wallet_file = std::env::var("HOME").unwrap() + "/.siertrichain/wallet.json";
    let wallet_data: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&wallet_file).expect("No wallet found")
    ).expect("Failed to parse wallet");
    
    let my_address = wallet_data["address"].as_str().unwrap();
    
    let db = Database::open("siertrichain.db").expect("Failed to open database");
    let chain = db.load_blockchain().expect("Failed to load blockchain");
    
    println!("💰 Wallet Balance\n");
    println!("📍 Address: {}", my_address);
    println!("📊 Chain Height: {}", chain.blocks.last().unwrap().height);
    println!("\n🔺 Your Triangles:");
    
    let mut my_triangles = 0;
    let mut total_area = 0.0;
    
    for (hash, triangle) in &chain.state.utxo_set {
        my_triangles += 1;
        total_area += triangle.area();
        println!("  • {} (area: {:.6})", &hash[..16], triangle.area());
    }
    
    println!("\n📈 Total: {} triangles", my_triangles);
    println!("📐 Combined area: {:.6}", total_area);
}
