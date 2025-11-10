use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router, http::StatusCode, response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tower_http::cors::{Any, CorsLayer};

use crate::blockchain::{Blockchain, Block};
use crate::persistence::Database;
use crate::transaction::Transaction;
use crate::crypto::KeyPair;

#[derive(Clone)]
struct AppState {
    blockchain: Arc<Mutex<Blockchain>>,
}

pub async fn run_api_server() {
    let db = Database::open("siertrichain.db").unwrap();
    let blockchain = db.load_blockchain().unwrap();

    let app_state = AppState {
        blockchain: Arc::new(Mutex::new(blockchain)),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // Blockchain endpoints
        .route("/blockchain/height", get(get_blockchain_height))
        .route("/blockchain/stats", get(get_blockchain_stats))
        .route("/blockchain/blocks", get(get_recent_blocks))
        .route("/blockchain/block/:hash", get(get_block_by_hash))
        .route("/blockchain/block/by-height/:height", get(get_block_by_height))
        .route("/blockchain/reward/:height", get(get_block_reward_info))
        // Address & Balance
        .route("/address/:addr/balance", get(get_address_balance))
        .route("/address/:addr/triangles", get(get_address_triangles))
        .route("/address/:addr/history", get(get_address_history))
        // Transactions
        .route("/transaction", post(submit_transaction))
        .route("/transaction/:hash", get(get_transaction_status))
        .route("/transactions/pending", get(get_pending_transactions))
        .route("/transactions/mempool-stats", get(get_mempool_stats))
        // Wallet
        .route("/wallet/create", post(create_wallet))
        .route("/wallet/import", post(import_wallet))
        // Mining
        .route("/mining/status", get(get_mining_status))
        .route("/mining/start", post(start_mining))
        .route("/mining/stop", post(stop_mining))
        // Network
        .route("/network/peers", get(get_peers))
        .route("/network/info", get(get_network_info))
        .with_state(app_state)
        .layer(cors);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn get_blockchain_height(State(state): State<AppState>) -> Json<u64> {
    let blockchain = state.blockchain.lock().unwrap();
    Json(blockchain.blocks.len() as u64)
}

async fn get_block_by_hash(State(state): State<AppState>, Path(hash): Path<String>) -> Result<Json<Option<Block>>, Response> {
    let blockchain = state.blockchain.lock().unwrap();
    let hash_bytes = match hex::decode(hash) {
        Ok(bytes) => bytes,
        Err(_) => return Err((StatusCode::BAD_REQUEST, "Invalid hash format").into_response()),
    };
    let mut hash_arr = [0u8; 32];
    if hash_bytes.len() != 32 {
        return Err((StatusCode::BAD_REQUEST, "Invalid hash length").into_response());
    }
    hash_arr.copy_from_slice(&hash_bytes);
    let block = blockchain.block_index.get(&hash_arr).cloned();
    Ok(Json(block))
}

#[derive(Serialize, Deserialize)]
pub struct BalanceResponse {
    pub triangles: Vec<String>,
    pub total_area: f64,
}

#[derive(Serialize, Deserialize)]
pub struct RecentBlock {
    pub height: u64,
    pub hash: String,
}

#[derive(Serialize, Deserialize)]
pub struct StatsResponse {
    pub height: u64,
    pub difficulty: u64,
    pub utxo_count: usize,
    pub mempool_size: usize,
    pub recent_blocks: Vec<RecentBlock>,
}

async fn get_blockchain_stats(State(state): State<AppState>) -> Json<StatsResponse> {
    let blockchain = state.blockchain.lock().unwrap();
    let recent_blocks = blockchain.blocks.iter().rev().take(6).map(|b| RecentBlock {
        height: b.header.height,
        hash: hex::encode(b.hash),
    }).collect();

    Json(StatsResponse {
        height: blockchain.blocks.len() as u64,
        difficulty: blockchain.difficulty,
        utxo_count: blockchain.state.utxo_set.len(),
        mempool_size: blockchain.mempool.len(),
        recent_blocks,
    })
}

async fn get_address_balance(State(state): State<AppState>, Path(addr): Path<String>) -> Json<BalanceResponse> {
    let blockchain = state.blockchain.lock().unwrap();
    let mut triangles = Vec::new();
    let mut total_area = 0.0;

    for (hash, triangle) in &blockchain.state.utxo_set {
        if triangle.owner == addr {
            triangles.push(hex::encode(hash));
            total_area += triangle.area();
        }
    }

    Json(BalanceResponse {
        triangles,
        total_area,
    })
}

async fn submit_transaction(State(state): State<AppState>, Json(tx): Json<Transaction>) -> Json<String> {
    let mut blockchain = state.blockchain.lock().unwrap();
    let tx_hash = tx.hash_str();
    blockchain.mempool.add_transaction(tx).unwrap();
    Json(tx_hash)
}

async fn get_transaction_status(State(state): State<AppState>, Path(hash): Path<String>) -> Result<Json<Option<Transaction>>, Response> {
    let blockchain = state.blockchain.lock().unwrap();
    let hash_bytes = match hex::decode(hash) {
        Ok(bytes) => bytes,
        Err(_) => return Err((StatusCode::BAD_REQUEST, "Invalid hash format").into_response()),
    };
    let mut hash_arr = [0u8; 32];
    if hash_bytes.len() != 32 {
        return Err((StatusCode::BAD_REQUEST, "Invalid hash length").into_response());
    }
    hash_arr.copy_from_slice(&hash_bytes);
    if let Some(tx) = blockchain.mempool.get_transaction(&hash_arr).cloned() {
        return Ok(Json(Some(tx)));
    }

    for block in &blockchain.blocks {
        if let Some(tx) = block.transactions.iter().find(|tx| tx.hash() == hash_arr) {
            return Ok(Json(Some(tx.clone())));
        }
    }

    Ok(Json(None))
}

// New endpoints

async fn get_recent_blocks(State(state): State<AppState>) -> Json<Vec<RecentBlock>> {
    let blockchain = state.blockchain.lock().unwrap();
    let blocks = blockchain.blocks.iter().rev().take(20).map(|b| RecentBlock {
        height: b.header.height,
        hash: hex::encode(b.hash),
    }).collect();
    Json(blocks)
}

async fn get_block_by_height(State(state): State<AppState>, Path(height): Path<u64>) -> Result<Json<Option<Block>>, Response> {
    let blockchain = state.blockchain.lock().unwrap();
    let block = blockchain.blocks.iter().find(|b| b.header.height == height).cloned();
    Ok(Json(block))
}

#[derive(Serialize, Deserialize)]
pub struct TriangleInfo {
    pub hash: String,
    pub area: f64,
    pub vertices: Vec<(f64, f64)>,
}

async fn get_address_triangles(State(state): State<AppState>, Path(addr): Path<String>) -> Json<Vec<TriangleInfo>> {
    let blockchain = state.blockchain.lock().unwrap();
    let triangles: Vec<TriangleInfo> = blockchain.state.utxo_set.iter()
        .filter(|(_, triangle)| triangle.owner == addr)
        .map(|(hash, triangle)| TriangleInfo {
            hash: hex::encode(hash),
            area: triangle.area(),
            vertices: vec![
                (triangle.a.x, triangle.a.y),
                (triangle.b.x, triangle.b.y),
                (triangle.c.x, triangle.c.y),
            ],
        })
        .collect();
    Json(triangles)
}

#[derive(Serialize, Deserialize)]
pub struct TransactionHistory {
    pub tx_hash: String,
    pub block_height: u64,
    pub timestamp: i64,
    pub tx_type: String,
}

async fn get_address_history(State(state): State<AppState>, Path(addr): Path<String>) -> Json<Vec<TransactionHistory>> {
    let blockchain = state.blockchain.lock().unwrap();
    let mut history = Vec::new();

    for block in &blockchain.blocks {
        for tx in &block.transactions {
            let involves_address = match tx {
                Transaction::Subdivision(tx) => tx.owner_address == addr,
                Transaction::Transfer(tx) => tx.sender == addr || tx.new_owner == addr,
                Transaction::Coinbase(tx) => tx.beneficiary_address == addr,
            };

            if involves_address {
                history.push(TransactionHistory {
                    tx_hash: tx.hash_str(),
                    block_height: block.header.height,
                    timestamp: block.header.timestamp,
                    tx_type: match tx {
                        Transaction::Subdivision(_) => "Subdivision".to_string(),
                        Transaction::Transfer(_) => "Transfer".to_string(),
                        Transaction::Coinbase(_) => "Coinbase".to_string(),
                    },
                });
            }
        }
    }

    Json(history)
}

async fn get_pending_transactions(State(state): State<AppState>) -> Json<Vec<Transaction>> {
    let blockchain = state.blockchain.lock().unwrap();
    Json(blockchain.mempool.get_all_transactions())
}

#[derive(Serialize, Deserialize)]
pub struct WalletResponse {
    pub address: String,
    pub public_key: String,
    pub private_key: String,
}

async fn create_wallet() -> Result<Json<WalletResponse>, Response> {
    match KeyPair::generate() {
        Ok(keypair) => {
            let address = keypair.address();
            let public_key = hex::encode(keypair.public_key.serialize());
            let private_key = hex::encode(keypair.secret_key.secret_bytes());

            Ok(Json(WalletResponse {
                address,
                public_key,
                private_key,
            }))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to generate keypair: {}", e)).into_response()),
    }
}

#[derive(Serialize, Deserialize)]
pub struct ImportWalletRequest {
    pub private_key: String,
}

async fn import_wallet(Json(req): Json<ImportWalletRequest>) -> Result<Json<WalletResponse>, Response> {
    let private_key_bytes = match hex::decode(&req.private_key) {
        Ok(bytes) => bytes,
        Err(_) => return Err((StatusCode::BAD_REQUEST, "Invalid private key format").into_response()),
    };

    match KeyPair::from_secret_bytes(&private_key_bytes) {
        Ok(keypair) => {
            let address = keypair.address();
            let public_key = hex::encode(keypair.public_key.serialize());

            Ok(Json(WalletResponse {
                address,
                public_key,
                private_key: req.private_key,
            }))
        }
        Err(e) => Err((StatusCode::BAD_REQUEST, format!("Invalid private key: {}", e)).into_response()),
    }
}

#[derive(Serialize, Deserialize)]
pub struct MiningStatus {
    pub is_mining: bool,
    pub blocks_mined: u64,
    pub hashrate: f64,
}

async fn get_mining_status(State(_state): State<AppState>) -> Json<MiningStatus> {
    // Placeholder - would need mining state
    Json(MiningStatus {
        is_mining: false,
        blocks_mined: 0,
        hashrate: 0.0,
    })
}

async fn start_mining(State(_state): State<AppState>) -> Json<String> {
    Json("Mining started (not implemented in API yet)".to_string())
}

async fn stop_mining(State(_state): State<AppState>) -> Json<String> {
    Json("Mining stopped".to_string())
}

#[derive(Serialize, Deserialize)]
pub struct PeerInfo {
    pub address: String,
    pub last_seen: i64,
}

async fn get_peers(State(_state): State<AppState>) -> Json<Vec<PeerInfo>> {
    // Placeholder - would need network state
    Json(vec![])
}

#[derive(Serialize, Deserialize)]
pub struct NetworkInfo {
    pub peers_count: usize,
    pub node_id: String,
    pub listening_port: u16,
}

async fn get_network_info(State(_state): State<AppState>) -> Json<NetworkInfo> {
    Json(NetworkInfo {
        peers_count: 0,
        node_id: "local-node".to_string(),
        listening_port: 8333,
    })
}

// New endpoints for enhanced block explorer functionality

#[derive(Serialize)]
struct MempoolStatsResponse {
    transaction_count: usize,
    total_fees: u64,
    avg_fee: f64,
    highest_fee: u64,
    lowest_fee: u64,
}

async fn get_mempool_stats(State(state): State<AppState>) -> Json<MempoolStatsResponse> {
    let blockchain = state.blockchain.lock().unwrap();
    let txs = blockchain.mempool.get_all_transactions();

    let fees: Vec<u64> = txs.iter().map(|tx| tx.fee()).collect();
    let total_fees: u64 = fees.iter().sum();
    let avg_fee = if !fees.is_empty() {
        total_fees as f64 / fees.len() as f64
    } else {
        0.0
    };
    let highest_fee = fees.iter().max().copied().unwrap_or(0);
    let lowest_fee = fees.iter().min().copied().unwrap_or(0);

    Json(MempoolStatsResponse {
        transaction_count: txs.len(),
        total_fees,
        avg_fee,
        highest_fee,
        lowest_fee,
    })
}

#[derive(Serialize)]
struct RewardInfoResponse {
    current_height: u64,
    current_reward: u64,
    next_halving_height: u64,
    blocks_until_halving: u64,
    reward_after_halving: u64,
}

async fn get_block_reward_info(State(state): State<AppState>, Path(height): Path<u64>) -> Json<RewardInfoResponse> {
    let blockchain = state.blockchain.lock().unwrap();
    let current_height = blockchain.blocks.len() as u64;
    let query_height = if height == 0 { current_height } else { height };

    let current_reward = Blockchain::calculate_block_reward(query_height);
    let halving_interval = 210_000u64;
    let next_halving_height = ((query_height / halving_interval) + 1) * halving_interval;
    let blocks_until_halving = next_halving_height.saturating_sub(query_height);
    let reward_after_halving = Blockchain::calculate_block_reward(next_halving_height);

    Json(RewardInfoResponse {
        current_height: query_height,
        current_reward,
        next_halving_height,
        blocks_until_halving,
        reward_after_halving,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum_test::TestServer;

    fn test_app() -> Router {
        let blockchain = Blockchain::new();
        let app_state = AppState {
            blockchain: Arc::new(Mutex::new(blockchain)),
        };

        Router::new()
            .route("/blockchain/height", get(get_blockchain_height))
            .route("/blockchain/block/:hash", get(get_block_by_hash))
            .route("/address/:addr/balance", get(get_address_balance))
            .route("/transaction", post(submit_transaction))
            .route("/transaction/:hash", get(get_transaction_status))
            .with_state(app_state)
    }

    #[tokio::test]
    async fn test_get_blockchain_height() {
        let server = TestServer::new(test_app()).unwrap();
        let response = server.get("/blockchain/height").await;
        assert_eq!(response.status_code(), StatusCode::OK);
        assert_eq!(response.json::<u64>(), 1);
    }

    #[tokio::test]
    async fn test_get_block_by_hash() {
        let server = TestServer::new(test_app()).unwrap();
        let genesis_hash = "0000000000000000000000000000000000000000000000000000000000000000";
        let response = server.get(&format!("/blockchain/block/{}", genesis_hash)).await;
        assert_eq!(response.status_code(), StatusCode::OK);
        let block: Option<Block> = response.json();
        assert!(block.is_some());
        assert_eq!(block.unwrap().hash, [0; 32]);
    }

    use crate::transaction::SubdivisionTx;
    use crate::crypto::KeyPair;

    #[tokio::test]
    async fn test_get_address_balance() {
        let server = TestServer::new(test_app()).unwrap();
        let genesis_owner = "genesis_owner";
        let response = server.get(&format!("/address/{}/balance", genesis_owner)).await;
        assert_eq!(response.status_code(), StatusCode::OK);
        let balance: BalanceResponse = response.json();
        assert_eq!(balance.triangles.len(), 1);
        assert!(balance.total_area > 0.0);
    }

    #[tokio::test]
    async fn test_submit_and_get_transaction() {
        let server = TestServer::new(test_app()).unwrap();
        let blockchain = Blockchain::new();
        let _genesis = blockchain.blocks[0].clone();
        let keypair = KeyPair::generate().unwrap();
        let address = keypair.address();
        let parent_hash = *blockchain.state.utxo_set.keys().next().unwrap();
        let children = blockchain.state.utxo_set.values().next().unwrap().subdivide();
        let mut tx = SubdivisionTx::new(parent_hash, children.to_vec(), address, 0, 1);
        let message = tx.signable_message();
        let signature = keypair.sign(&message).unwrap();
        let public_key = keypair.public_key.serialize().to_vec();
        tx.sign(signature, public_key);
        let transaction = Transaction::Subdivision(tx);

        let response = server.post("/transaction").json(&transaction).await;
        assert_eq!(response.status_code(), StatusCode::OK);
        let tx_hash: String = response.json();
        assert!(!tx_hash.is_empty());

        let response = server.get(&format!("/transaction/{}", tx_hash)).await;
        assert_eq!(response.status_code(), StatusCode::OK);
        let tx_status: Option<Transaction> = response.json();
        assert!(tx_status.is_some());
    }
}
