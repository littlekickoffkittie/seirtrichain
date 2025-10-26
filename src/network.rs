//! P2P Networking for siertrichain

use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::blockchain::Blockchain;
use crate::persistence::Database;
use crate::error::ChainError;

#[derive(Debug, Clone)]
pub struct Node {
    pub host: String,
    pub port: u16,
}

impl Node {
    pub fn new(host: String, port: u16) -> Self {
        Node { host, port }
    }
    
    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

pub struct NetworkNode {
    blockchain: Arc<RwLock<Blockchain>>,
    db_path: String,
    peers: Arc<RwLock<Vec<Node>>>,
}

impl NetworkNode {
    pub fn new(blockchain: Blockchain, db_path: String) -> Self {
        NetworkNode {
            blockchain: Arc::new(RwLock::new(blockchain)),
            db_path,
            peers: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub async fn start_server(&self, port: u16) -> Result<(), ChainError> {
        let addr = format!("0.0.0.0:{}", port);
        let listener = TcpListener::bind(&addr).await
            .map_err(|e| ChainError::NetworkError(format!("Failed to bind: {}", e)))?;
        
        println!("🌐 Node listening on {}", addr);
        
        loop {
            match listener.accept().await {
                Ok((socket, peer_addr)) => {
                    println!("📡 New connection from {}", peer_addr);
                    let blockchain = self.blockchain.clone();
                    
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(socket, blockchain).await {
                            eprintln!("❌ Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("❌ Accept error: {}", e);
                }
            }
        }
    }
    
    pub async fn connect_peer(&self, host: String, port: u16) -> Result<(), ChainError> {
        let addr = format!("{}:{}", host, port);
        println!("🔗 Connecting to peer: {}", addr);
        
        let mut stream = TcpStream::connect(&addr).await
            .map_err(|e| ChainError::NetworkError(format!("Failed to connect: {}", e)))?;
        
        let request = NetworkMessage::GetBlockchain;
        let data = bincode::serialize(&request)
            .map_err(|e| ChainError::NetworkError(format!("Serialization failed: {}", e)))?;
        
        let len = data.len() as u32;
        stream.write_all(&len.to_be_bytes()).await
            .map_err(|e| ChainError::NetworkError(format!("Write failed: {}", e)))?;
        stream.write_all(&data).await
            .map_err(|e| ChainError::NetworkError(format!("Write failed: {}", e)))?;
        
        let mut len_bytes = [0u8; 4];
        stream.read_exact(&mut len_bytes).await
            .map_err(|e| ChainError::NetworkError(format!("Read failed: {}", e)))?;
        let len = u32::from_be_bytes(len_bytes) as usize;
        
        let mut buffer = vec![0u8; len];
        stream.read_exact(&mut buffer).await
            .map_err(|e| ChainError::NetworkError(format!("Read failed: {}", e)))?;
        
        let response: NetworkMessage = bincode::deserialize(&buffer)
            .map_err(|e| ChainError::NetworkError(format!("Deserialization failed: {}", e)))?;
        
        match response {
            NetworkMessage::Blockchain(remote_chain) => {
                let mut chain = self.blockchain.write().await;
                
                if remote_chain.blocks.len() > chain.blocks.len() {
                    println!("📥 Syncing blockchain (height: {} -> {})", 
                        chain.blocks.len() - 1, 
                        remote_chain.blocks.len() - 1
                    );
                    
                    *chain = remote_chain.clone();
                    
                    // Save to database
                    let db = Database::open(&self.db_path)
                        .map_err(|e| ChainError::NetworkError(format!("DB open failed: {}", e)))?;
                    
                    for block in &chain.blocks {
                        db.save_block(block)
                            .map_err(|e| ChainError::NetworkError(format!("DB save failed: {}", e)))?;
                    }
                    db.save_utxo_set(&chain.state)
                        .map_err(|e| ChainError::NetworkError(format!("DB save failed: {}", e)))?;
                    db.save_difficulty(chain.difficulty)
                        .map_err(|e| ChainError::NetworkError(format!("DB save failed: {}", e)))?;
                    
                    println!("✅ Blockchain synced!");
                } else {
                    println!("✅ Already up to date");
                }
            }
            _ => {
                return Err(ChainError::NetworkError("Unexpected response".to_string()));
            }
        }
        
        let mut peers = self.peers.write().await;
        let peer = Node::new(host, port);
        if !peers.iter().any(|p| p.addr() == peer.addr()) {
            peers.push(peer);
        }
        
        Ok(())
    }
    
    pub async fn get_height(&self) -> u64 {
        let chain = self.blockchain.read().await;
        chain.blocks.last().map(|b| b.height).unwrap_or(0)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
enum NetworkMessage {
    GetBlockchain,
    Blockchain(Blockchain),
    Ping,
    Pong,
}

async fn handle_connection(
    mut socket: TcpStream,
    blockchain: Arc<RwLock<Blockchain>>,
) -> Result<(), ChainError> {
    let mut len_bytes = [0u8; 4];
    socket.read_exact(&mut len_bytes).await
        .map_err(|e| ChainError::NetworkError(format!("Read failed: {}", e)))?;
    let len = u32::from_be_bytes(len_bytes) as usize;
    
    let mut buffer = vec![0u8; len];
    socket.read_exact(&mut buffer).await
        .map_err(|e| ChainError::NetworkError(format!("Read failed: {}", e)))?;
    
    let message: NetworkMessage = bincode::deserialize(&buffer)
        .map_err(|e| ChainError::NetworkError(format!("Deserialization failed: {}", e)))?;
    
    match message {
        NetworkMessage::GetBlockchain => {
            let chain = blockchain.read().await;
            let response = NetworkMessage::Blockchain(chain.clone());
            let data = bincode::serialize(&response)
                .map_err(|e| ChainError::NetworkError(format!("Serialization failed: {}", e)))?;
            
            let len = data.len() as u32;
            socket.write_all(&len.to_be_bytes()).await
                .map_err(|e| ChainError::NetworkError(format!("Write failed: {}", e)))?;
            socket.write_all(&data).await
                .map_err(|e| ChainError::NetworkError(format!("Write failed: {}", e)))?;
            
            println!("📤 Sent blockchain to peer");
        }
        NetworkMessage::Ping => {
            let response = NetworkMessage::Pong;
            let data = bincode::serialize(&response)
                .map_err(|e| ChainError::NetworkError(format!("Serialization failed: {}", e)))?;
            
            let len = data.len() as u32;
            socket.write_all(&len.to_be_bytes()).await
                .map_err(|e| ChainError::NetworkError(format!("Write failed: {}", e)))?;
            socket.write_all(&data).await
                .map_err(|e| ChainError::NetworkError(format!("Write failed: {}", e)))?;
        }
        _ => {}
    }
    
    Ok(())
}
