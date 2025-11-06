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
        .route("/blockchain/height", get(get_blockchain_height))
        .route("/blockchain/stats", get(get_blockchain_stats))
        .route("/blockchain/block/:hash", get(get_block_by_hash))
        .route("/address/:addr/balance", get(get_address_balance))
        .route("/transaction", post(submit_transaction))
        .route("/transaction/:hash", get(get_transaction_status))
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum_test::TestServer;
    use crate::blockchain::Sha256Hash;

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
        let mut blockchain = Blockchain::new();
        let genesis = blockchain.blocks[0].clone();
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
