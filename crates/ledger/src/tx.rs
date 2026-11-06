use set_core::{Tx, TxEntry};
use std::path::Path;

pub struct TxLedger {
    pub log: crate::DualLog<TxEntry>,
    pub entries: Vec<TxEntry>,
    next_id: u64,
}

impl TxLedger {
    pub fn new(dir: &Path, name: &str) -> Self {
        let log = crate::DualLog::new(dir, name);
        let entries: Vec<TxEntry> = log.read_all().unwrap_or_default();
        let next_id = entries.iter().map(|e| e.id).max().unwrap_or(0) + 1;
        Self {
            log,
            entries,
            next_id,
        }
    }
    pub fn append_tx(&mut self, tx: Tx) -> Result<u64, String> {
        let id = self.next_id;
        self.next_id += 1;
        let e = TxEntry { id, tx };
        self.log.append(&e).map_err(|e| e.to_string())?;
        self.entries.push(e);
        Ok(id)
    }
    pub fn contains_tx(&self, id: u64) -> bool {
        self.entries.iter().any(|e| e.id == id)
    }
    pub fn get(&self, id: u64) -> Option<&TxEntry> {
        self.entries.iter().find(|e| e.id == id)
    }
}
