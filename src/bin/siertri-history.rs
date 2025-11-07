//! View transaction history for your wallet

use siertrichain::persistence::Database;
use siertrichain::transaction::Transaction;

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

    println!("📜 Transaction History\n");
    println!("📍 Your Address: {}...\n", &my_address[..42.min(my_address.len())]);

    let mut tx_count = 0;
    let mut received_count = 0;
    let mut sent_count = 0;

    // Iterate through all blocks
    for block in &chain.blocks {
        for tx in &block.transactions {
            match tx {
                Transaction::Transfer(transfer_tx) => {
                    let is_sender = transfer_tx.sender == my_address;
                    let is_receiver = transfer_tx.new_owner == my_address;

                    if is_sender || is_receiver {
                        tx_count += 1;

                        let direction = if is_sender && is_receiver {
                            "↔️ Self"
                        } else if is_sender {
                            sent_count += 1;
                            "📤 Sent"
                        } else {
                            received_count += 1;
                            "📥 Received"
                        };

                        println!("─────────────────────────────────────────");
                        println!("{} | Block #{}", direction, block.header.height);
                        let hash_hex = hex::encode(transfer_tx.input_hash);
                        println!("  🔺 Triangle: {}", &hash_hex[..16.min(hash_hex.len())]);

                        if is_sender {
                            let to_addr = if transfer_tx.new_owner.len() >= 16 {
                                &transfer_tx.new_owner[..16]
                            } else {
                                &transfer_tx.new_owner
                            };
                            println!("  📬 To: {}...", to_addr);
                        } else {
                            let from_addr = if transfer_tx.sender.len() >= 16 {
                                &transfer_tx.sender[..16]
                            } else {
                                &transfer_tx.sender
                            };
                            println!("  📨 From: {}...", from_addr);
                        }

                        if let Some(memo) = &transfer_tx.memo {
                            println!("  📝 Memo: {}", memo);
                        }

                        println!("  ⏰ Time: {}", format_timestamp(block.header.timestamp));
                    }
                }
                Transaction::Coinbase(coinbase_tx) => {
                    if coinbase_tx.beneficiary_address == my_address {
                        tx_count += 1;
                        received_count += 1;

                        println!("─────────────────────────────────────────");
                        println!("⛏️  Mining Reward | Block #{}", block.header.height);
                        println!("  📐 Reward Area: {}", coinbase_tx.reward_area);
                        println!("  ⏰ Time: {}", format_timestamp(block.header.timestamp));
                    }
                }
                Transaction::Subdivision(sub_tx) => {
                    if sub_tx.owner_address == my_address {
                        tx_count += 1;

                        println!("─────────────────────────────────────────");
                        println!("✂️  Subdivision | Block #{}", block.header.height);
                        let hash_hex = hex::encode(sub_tx.parent_hash);
                        println!("  🔺 Parent: {}", &hash_hex[..16.min(hash_hex.len())]);
                        println!("  👶 Children: {} triangles", sub_tx.children.len());
                        println!("  ⏰ Time: {}", format_timestamp(block.header.timestamp));
                    }
                }
            }
        }
    }

    println!("─────────────────────────────────────────");
    println!("\n📊 Summary:");
    println!("  Total Transactions: {}", tx_count);
    println!("  📥 Received: {}", received_count);
    println!("  📤 Sent: {}", sent_count);
    println!();

    Ok(())
}

fn format_timestamp(timestamp: i64) -> String {
    use chrono::DateTime;

    if let Some(dt) = DateTime::from_timestamp(timestamp, 0) {
        dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
    } else {
        format!("Invalid timestamp: {}", timestamp)
    }
}
