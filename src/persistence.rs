//! Database persistence layer for siertrichain

use rusqlite::{Connection, params};
use crate::blockchain::{Blockchain, Block, BlockHeader, TriangleState};
use crate::transaction::Transaction;
use crate::geometry::Triangle;
use crate::error::ChainError;
use std::collections::HashMap;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &str) -> Result<Self, ChainError> {
        let conn = Connection::open(path)
            .map_err(|e| ChainError::DatabaseError(format!("Failed to open database: {}", e)))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS blocks (
                height INTEGER PRIMARY KEY,
                hash TEXT NOT NULL,
                previous_hash TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                difficulty INTEGER NOT NULL,
                nonce INTEGER NOT NULL,
                merkle_root TEXT NOT NULL,
                transactions TEXT NOT NULL
            )",
            [],
        ).map_err(|e| ChainError::DatabaseError(format!("Failed to create blocks table: {}", e)))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS utxo_set (
                hash TEXT PRIMARY KEY,
                triangle_data TEXT NOT NULL
            )",
            [],
        ).map_err(|e| ChainError::DatabaseError(format!("Failed to create utxo_set table: {}", e)))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        ).map_err(|e| ChainError::DatabaseError(format!("Failed to create metadata table: {}", e)))?;

        Ok(Database { conn })
    }

    pub fn save_block(&self, block: &Block) -> Result<(), ChainError> {
        let transactions_json = serde_json::to_string(&block.transactions)
            .map_err(|e| ChainError::DatabaseError(format!("Failed to serialize transactions: {}", e)))?;

        self.conn.execute(
            "INSERT OR REPLACE INTO blocks (height, hash, previous_hash, timestamp, difficulty, nonce, merkle_root, transactions)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                block.height as i64,
                block.hash,
                block.previous_hash,
                block.timestamp,
                block.difficulty as i64,
                block.nonce as i64,
                block.merkle_root,
                transactions_json,
            ],
        ).map_err(|e| ChainError::DatabaseError(format!("Failed to save block: {}", e)))?;

        Ok(())
    }

    pub fn save_utxo_set(&self, state: &TriangleState) -> Result<(), ChainError> {
        self.conn.execute("DELETE FROM utxo_set", [])
            .map_err(|e| ChainError::DatabaseError(format!("Failed to clear utxo_set: {}", e)))?;

        for (hash, triangle) in &state.utxo_set {
            let triangle_json = serde_json::to_string(triangle)
                .map_err(|e| ChainError::DatabaseError(format!("Failed to serialize triangle: {}", e)))?;

            self.conn.execute(
                "INSERT INTO utxo_set (hash, triangle_data) VALUES (?1, ?2)",
                params![hash, triangle_json],
            ).map_err(|e| ChainError::DatabaseError(format!("Failed to save UTXO: {}", e)))?;
        }

        Ok(())
    }

    pub fn save_difficulty(&self, difficulty: u64) -> Result<(), ChainError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO metadata (key, value) VALUES ('difficulty', ?1)",
            params![difficulty.to_string()],
        ).map_err(|e| ChainError::DatabaseError(format!("Failed to save difficulty: {}", e)))?;

        Ok(())
    }

    pub fn load_blockchain(&self) -> Result<Blockchain, ChainError> {
        let mut stmt = self.conn.prepare(
            "SELECT height, hash, previous_hash, timestamp, difficulty, nonce, merkle_root, transactions
             FROM blocks ORDER BY height ASC"
        ).map_err(|e| ChainError::DatabaseError(format!("Failed to prepare query: {}", e)))?;

        let blocks_iter = stmt.query_map([], |row| {
            let transactions_json: String = row.get(7)?;
            let transactions: Vec<Transaction> = serde_json::from_str(&transactions_json)
                .map_err(|e| rusqlite::Error::InvalidQuery)?;

            let height: i64 = row.get(0)?;
            let timestamp: i64 = row.get(3)?;
            let difficulty: i64 = row.get(4)?;
            let nonce: i64 = row.get(5)?;
            let hash: String = row.get(1)?;
            let previous_hash: String = row.get(2)?;
            let merkle_root: String = row.get(6)?;

            Ok(Block {
                header: BlockHeader {
                    height: height as u64,
                    previous_hash: previous_hash.clone(),
                    timestamp,
                    difficulty: difficulty as u64,
                    nonce: nonce as u64,
                    merkle_root: merkle_root.clone(),
                },
                hash,
                height: height as u64,
                previous_hash,
                timestamp,
                difficulty: difficulty as u64,
                nonce: nonce as u64,
                merkle_root,
                transactions,
            })
        }).map_err(|e| ChainError::DatabaseError(format!("Failed to query blocks: {}", e)))?;

        let mut blocks = Vec::new();
        for block_result in blocks_iter {
            blocks.push(block_result.map_err(|e| ChainError::DatabaseError(format!("Failed to load block: {}", e)))?);
        }

        if blocks.is_empty() {
            return Err(ChainError::DatabaseError("No blocks in database".to_string()));
        }

        let mut utxo_set = HashMap::new();
        let mut stmt = self.conn.prepare("SELECT hash, triangle_data FROM utxo_set")
            .map_err(|e| ChainError::DatabaseError(format!("Failed to prepare UTXO query: {}", e)))?;

        let utxo_iter = stmt.query_map([], |row| {
            let hash: String = row.get(0)?;
            let triangle_json: String = row.get(1)?;
            let triangle: Triangle = serde_json::from_str(&triangle_json)
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            Ok((hash, triangle))
        }).map_err(|e| ChainError::DatabaseError(format!("Failed to query UTXOs: {}", e)))?;

        for utxo_result in utxo_iter {
            let (hash, triangle) = utxo_result.map_err(|e| ChainError::DatabaseError(format!("Failed to load UTXO: {}", e)))?;
            utxo_set.insert(hash, triangle);
        }

        let difficulty: u64 = self.conn.query_row(
            "SELECT value FROM metadata WHERE key = 'difficulty'",
            [],
            |row| {
                let val: String = row.get(0)?;
                Ok(val.parse::<u64>().unwrap_or(2))
            }
        ).unwrap_or(2);

        Ok(Blockchain {
            blocks,
            state: TriangleState { utxo_set },
            difficulty,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::Blockchain;

    #[test]
    fn test_database_open() {
        let db = Database::open(":memory:").unwrap();
        assert!(db.conn.is_autocommit());
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
        assert_eq!(loaded_chain.difficulty, chain.difficulty);
    }
}
