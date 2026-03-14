use duckdb::{params, Connection, Result};
use std::sync::{Arc, Mutex};
use tracing::info;

pub struct Ledger {
    conn: Arc<Mutex<Connection>>,
}

impl Ledger {
    /// Initializes a new Ledger with a DuckDB file.
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        
        // Create initial tables
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS credits (
                peer_id TEXT PRIMARY KEY,
                balance BIGINT DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS transactions (
                id INTEGER PRIMARY KEY,
                from_peer TEXT,
                to_peer TEXT,
                amount BIGINT,
                timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );"
        )?;
        
        info!("Ledger initialized at {}", path);
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Gets the credit balance for a specific peer.
    pub fn get_balance(&self, peer_id: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT balance FROM credits WHERE peer_id = ?")?;
        let mut rows = stmt.query(params![peer_id])?;
        
        if let Some(row) = rows.next()? {
            Ok(row.get(0)?)
        } else {
            Ok(0)
        }
    }

    /// Adds credits to a peer (e.g., for contributing compute).
    pub fn add_credits(&self, peer_id: &str, amount: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO credits (peer_id, balance) 
             VALUES (?, ?) 
             ON CONFLICT(peer_id) DO UPDATE SET balance = balance + EXCLUDED.balance",
            params![peer_id, amount],
        )?;
        info!("Added {} credits to peer {}", amount, peer_id);
        Ok(())
    }

    /// Spends credits from a peer (e.g., for requesting inference).
    pub fn spend_credits(&self, peer_id: &str, amount: i64) -> Result<bool> {
        let current_balance = self.get_balance(peer_id)?;
        if current_balance < amount {
            return Ok(false);
        }

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE credits SET balance = balance - ? WHERE peer_id = ?",
            params![amount, peer_id],
        )?;
        info!("Peer {} spent {} credits", peer_id, amount);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ledger_initialization() {
        let ledger = Ledger::new(":memory:").unwrap();
        assert_eq!(ledger.get_balance("peer1").unwrap(), 0);
    }

    #[test]
    fn test_add_and_get_balance() {
        let ledger = Ledger::new(":memory:").unwrap();
        
        ledger.add_credits("peer1", 100).unwrap();
        assert_eq!(ledger.get_balance("peer1").unwrap(), 100);
        
        ledger.add_credits("peer1", 50).unwrap();
        assert_eq!(ledger.get_balance("peer1").unwrap(), 150);
    }

    #[test]
    fn test_spend_credits() {
        let ledger = Ledger::new(":memory:").unwrap();
        
        ledger.add_credits("peer1", 100).unwrap();
        
        // Successful spend
        let success = ledger.spend_credits("peer1", 40).unwrap();
        assert!(success);
        assert_eq!(ledger.get_balance("peer1").unwrap(), 60);
        
        // Insufficient funds
        let success = ledger.spend_credits("peer1", 100).unwrap();
        assert!(!success);
        assert_eq!(ledger.get_balance("peer1").unwrap(), 60);
    }
}
