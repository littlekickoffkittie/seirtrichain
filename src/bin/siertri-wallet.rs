//! Wallet CLI for siertrichain

use siertrichain::crypto::KeyPair;
use std::fs;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        print_usage();
        return;
    }
    
    match args[1].as_str() {
        "new" => create_wallet(),
        "address" => show_address(),
        "help" => print_usage(),
        _ => {
            println!("Unknown command: {}", args[1]);
            print_usage();
        }
    }
}

fn create_wallet() {
    let wallet_dir = std::env::var("HOME").unwrap() + "/.siertrichain";
    fs::create_dir_all(&wallet_dir).expect("Failed to create wallet directory");
    
    let keypair = KeyPair::generate().expect("Failed to generate keypair");
    let address = keypair.address();
    
    let wallet_file = wallet_dir.clone() + "/wallet.json";
    
    if std::path::Path::new(&wallet_file).exists() {
        println!("⚠️  Wallet already exists");
        println!("   Address: {}", address);
        return;
    }
    
    let secret_hex = hex::encode(keypair.secret_key.secret_bytes());
    let wallet_data = serde_json::json!({
        "address": address,
        "secret_key": secret_hex,
        "created": chrono::Utc::now().to_rfc3339(),
    });
    
    fs::write(&wallet_file, serde_json::to_string_pretty(&wallet_data).unwrap())
        .expect("Failed to write wallet file");
    
    println!("🔑 New wallet created!");
    println!("   Address: {}", address);
    println!("   Location: {}", wallet_file);
    println!("\n⚠️  IMPORTANT: Backup your wallet file!");
}

fn show_address() {
    let wallet_file = std::env::var("HOME").unwrap() + "/.siertrichain/wallet.json";
    
    if !std::path::Path::new(&wallet_file).exists() {
        println!("❌ No wallet found. Run 'siertri-wallet new' first.");
        return;
    }
    
    let wallet_data: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&wallet_file).expect("Failed to read wallet")
    ).expect("Failed to parse wallet");
    
    println!("📍 Your address: {}", wallet_data["address"].as_str().unwrap());
}

fn print_usage() {
    println!("🔺 siertri-wallet - Manage your siertrichain wallet\n");
    println!("Usage:");
    println!("  siertri-wallet new        Create a new wallet");
    println!("  siertri-wallet address    Show your wallet address");
    println!("  siertri-wallet help       Show this help message");
}
