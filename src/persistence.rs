//! Persistence layer for siertrichain
//! Saves blockchain state to SQLite database

use rusqlite::{Connection, params};
use crate::blockchain::{Block, Blockchain, TriangleState};
use crate::geometry::Triangle;
use crate::error::ChainError;
use std::path::Path;

pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create database at the given path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, ChainError> {
        let conn = Connection::open(path)
            .map_err(|e| ChainError::DatabaseError(format!("Failed to open database: {}", e)))?;
        
        let db = Database { conn };
        db.init_schema()?;
        Ok(db)
    }
    
    /// Create the database schema
    fn init_schema(&self) -> Result<(), ChainError> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS blocks (
                height INTEGER PRIMARY KEY,
                timestamp TEXT NOT NULL,
                previous_hash TEXT NOT NULL,
                merkle_root TEXT NOT NULL,
                difficulty INTEGER NOT NULL,
                nonce INTEGER NOT NULL,
                hash TEXT NOT NULL UNIQUE,
                tx_count INTEGER NOT NULL
            )",
            [],
        ).map_err(|e| ChainError::DatabaseError(format!("Failed to create blocks table: {}", e)))?;
        
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS utxo_set (
                triangle_hash TEXT PRIMARY KEY,
                triangle_data TEXT NOT NULL
            )",
            [],
        ).map_err(|e| ChainError::DatabaseError(format!("Failed to create utxo_set table: {}", e)))?;
        
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        ).map_err(|e| ChainError::DatabaseError(format!("Failed to create metadata table: {}", e)))?;
        
        Ok(())
    }
    
    /// Save a block to the database
    pub fn save_block(&self, block: &Block) -> Result<(), ChainError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO blocks (height, timestamp, previous_hash, merkle_root, difficulty, nonce, hash, tx_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                block.height,
                block.timestamp.to_rfc3339(),
                block.previous_hash,
                block.merkle_root,
                block.difficulty,
                block.nonce,
                block.hash,
                block.transactions.len() as i64,
            ],
        ).map_err(|e| ChainError::DatabaseError(format!("Failed to save block: {}", e)))?;
        
        Ok(())
    }
    
    /// Save the entire UTXO set
    pub fn save_utxo_set(&self, state: &TriangleState) -> Result<(), ChainError> {
        self.conn.execute("DELETE FROM utxo_set", [])
            .map_err(|e| ChainError::DatabaseError(format!("Failed to clear UTXO set: {}", e)))?;
        
        for (hash, triangle) in &state.utxo_set {
            let triangle_json = serde_json::to_string(triangle)
                .map_err(|e| ChainError::DatabaseError(format!("Failed to serialize triangle: {}", e)))?;
            
            self.conn.execute(
                "INSERT INTO utxo_set (triangle_hash, triangle_data) VALUES (?1, ?2)",
                params![hash, triangle_json],
            ).map_err(|e| ChainError::DatabaseError(format!("Failed to save triangle: {}", e)))?;
        }
        
        Ok(())
    }
    
    /// Save blockchain metadata
    pub fn save_difficulty(&self, difficulty: u64) -> Result<(), ChainError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO metadata (key, value) VALUES ('difficulty', ?1)",
            params![difficulty.to_string()],
        ).map_err(|e| ChainError::DatabaseError(format!("Failed to save difficulty: {}", e)))?;
        
        Ok(())
    }
    
    /// Load the entire blockchain from database
    pub fn load_blockchain(&self) -> Result<Blockchain, ChainError> {
        let mut chain = Blockchain {
            blocks: Vec::new(),
            state: TriangleState::new(),
            difficulty: 2,
        };
        
        let mut stmt = self.conn.prepare("SELECT height, timestamp, previous_hash, merkle_root, difficulty, nonce, hash FROM blocks ORDER BY height")
            .map_err(|e| ChainError::DatabaseError(format!("Failed to prepare block query: {}", e)))?;
        
        let blocks = stmt.query_map([], |row| {
            Ok(Block {
                height: row.get(0)?,
                timestamp: row.get::<_, String>(1)?.parse().unwrap(),
                previous_hash: row.get(2)?,
                merkle_root: row.get(3)?,
                difficulty: row.get(4)?,
                nonce: row.get(5)?,
                transactions: vec![],
                hash: row.get(6)?,
            })
        }).map_err(|e| ChainError::DatabaseError(format!("Failed to query blocks: {}", e)))?;
        
        for block in blocks {
            chain.blocks.push(block.map_err(|e| ChainError::DatabaseError(format!("Failed to load block: {}", e)))?);
        }
        
        let mut stmt = self.conn.prepare("SELECT triangle_hash, triangle_data FROM utxo_set")
            .map_err(|e| ChainError::DatabaseError(format!("Failed to prepare UTXO query: {}", e)))?;
        
        let utxos = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }).map_err(|e| ChainError::DatabaseError(format!("Failed to query UTXO set: {}", e)))?;
        
        for utxo in utxos {
            let (hash, triangle_json) = utxo.map_err(|e| ChainError::DatabaseError(format!("Failed to load UTXO: {}", e)))?;
            let triangle: Triangle = serde_json::from_str(&triangle_json)
                .map_err(|e| ChainError::DatabaseError(format!("Failed to deserialize triangle: {}", e)))?;
            chain.state.utxo_set.insert(hash, triangle);
        }
        
        let difficulty: Result<u64, _> = self.conn.query_row(
            "SELECT value FROM metadata WHERE key = 'difficulty'",
            [],
            |row| row.get::<_, String>(0)?.parse::<u64>().map_err(|_| rusqlite::Error::InvalidQuery)
        );
        
        if let Ok(diff) = difficulty {
            chain.difficulty = diff;
        }
        
        Ok(chain)
    }
    
    /// Get the current block height from database
    pub fn get_block_height(&self) -> Result<u64, ChainError> {
        let height: Result<u64, _> = self.conn.query_row(
            "SELECT MAX(height) FROM blocks",
            [],
            |row| row.get(0)
        );
        
        Ok(height.unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_database_open() {
        let db = Database::open(":memory:").unwrap();
        assert_eq!(db.get_block_height().unwrap(), 0);
    }
    
    #[test]
    fn test_save_and_load_blockchain() {
        let db = Database::open(":memory:").unwrap();
        let chain = Blockchain::new();
        
        db.save_block(&chain.blocks[0]).unwrap();
        db.save_utxo_set(&chain.state).unwrap();
        db.save_difficulty(chain.difficulty).unwrap();
        
        let loaded_chain = db.load_blockchain().unwrap();
        assert_eq!(loaded_chain.blocks.len(), 1);
        assert_eq!(loaded_chain.blocks[0].height, 0);
        assert_eq!(loaded_chain.state.count(), chain.state.count());
        assert_eq!(loaded_chain.difficulty, chain.difficulty);
    }
}
