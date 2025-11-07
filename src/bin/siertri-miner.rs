//! Miner CLI for siertrichain

use siertrichain::blockchain::{Blockchain, Block};
use siertrichain::persistence::Database;
use siertrichain::network::NetworkNode;
use siertrichain::transaction::{Transaction, CoinbaseTx};
use std::env;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: siertri-miner <beneficiary_address> [--peer <host:port>]");
        return;
    }
    let beneficiary_address = args[1].clone();

    println!("⛏️  siertri-miner v0.1.0\n");
    
    let db = Database::open("siertrichain.db").expect("Failed to open database");
    let mut chain = db.load_blockchain().unwrap_or_else(|_| {
        println!("⚠️  No blockchain found, creating genesis...");
        Blockchain::new()
    });
    
    let network_node = NetworkNode::new(chain.clone(), "siertrichain.db".to_string());

    if args.len() >= 4 && args[2] == "--peer" {
        let peer_addr = &args[3];
        let parts: Vec<&str> = peer_addr.split(':').collect();
        if parts.len() == 2 {
            let peer_host = parts[0].to_string();
            let peer_port: u16 = parts[1].parse().expect("Invalid peer port");
            
            println!("🔗 Connecting to peer {}:{}...", peer_host, peer_port);
            if let Err(e) = network_node.connect_peer(peer_host, peer_port).await {
                eprintln!("❌ Failed to connect to peer: {}", e);
            }
        }
    }

    loop {
        let last_block = chain.blocks.last().unwrap();
        let new_height = last_block.header.height + 1;
        let difficulty = chain.difficulty;

        let coinbase_tx = Transaction::Coinbase(CoinbaseTx {
            reward_area: 1000,
            beneficiary_address: beneficiary_address.clone(),
        });

        let mut new_block = Block::new(
            new_height,
            last_block.hash,
            difficulty,
            vec![coinbase_tx],
        );

        println!(" Mined a new block at height {}", new_height);

        loop {
            new_block.hash = new_block.calculate_hash();
            if new_block.verify_proof_of_work() {
                println!("  Found a valid hash: {}", hex::encode(new_block.hash));
                break;
            }
            new_block.header.nonce += 1;
        }

        if let Err(e) = chain.apply_block(new_block.clone()) {
            eprintln!("❌ Failed to apply new block: {}", e);
            sleep(Duration::from_secs(10)).await;
            continue;
        }

        db.save_block(&new_block).expect("Failed to save block");
        db.save_utxo_set(&chain.state).expect("Failed to save UTXO");
        db.save_difficulty(chain.difficulty).expect("Failed to save difficulty");

        println!("  Saved new block to the database");

        if let Err(e) = network_node.broadcast_block(&new_block).await {
            eprintln!("❌ Failed to broadcast block: {}", e);
        }
    }
}