use siertrichain::api::run_api_server;
use siertrichain::persistence::Database;
use siertrichain::blockchain::Blockchain;

#[tokio::main]
async fn main() {
    let db = Database::open("siertrichain.db").unwrap();
    if db.load_blockchain().is_err() {
        let chain = Blockchain::new();
        db.save_blockchain_state(&chain.blocks[0], &chain.state, chain.difficulty).unwrap();
    }

    println!("Starting the siertrichain API server...");
    run_api_server().await;
}
