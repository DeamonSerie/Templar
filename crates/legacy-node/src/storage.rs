use set_ledger::{PriceLedger, TxLedger};
use set_core::Tx;
use set_token::burn_tx_for;
use std::collections::BTreeMap;
use std::path::Path;

pub struct NodeStorage {
    pub tx: TxLedger,
    pub price: PriceLedger,
    pub tan_queries: BTreeMap<u64, Vec<u8>>,
    pub tan_results: BTreeMap<u64, Vec<u8>>,
}

impl NodeStorage {
    pub fn new(dir: &Path) -> Self {
        let _ = std::fs::create_dir_all(dir);
        Self {
            tx: TxLedger::new(dir, "tx"),
            price: PriceLedger::new(dir, "price"),
            tan_queries: BTreeMap::new(),
            tan_results: BTreeMap::new(),
        }
    }
    pub fn verify_copies(&self) -> bool {
        self.tx.log.read_all().is_ok() && self.price.log.read_all().is_ok()
    }

    pub fn accept_tx(&mut self, tx: Tx) -> Result<(), String> {
        self.tx.append_tx(tx.clone())?;
        if let Some(burn) = burn_tx_for(&tx) {
            self.tx.append_tx(burn)?;
        }
        Ok(())
    }
    pub fn submit_tan_query(&mut self, id: u64, raw: Vec<u8>) -> Result<(), String> {
        self.tan_queries.insert(id, raw);
        Ok(())
    }
    pub fn store_tan_result(&mut self, id: u64, result: Vec<u8>) -> Result<(), String> {
        self.tan_results.insert(id, result);
        Ok(())
    }
    pub fn tan_query(&self, id: u64) -> Option<&Vec<u8>> {
        self.tan_queries.get(&id)
    }
    pub fn tan_result(&self, id: u64) -> Option<&Vec<u8>> {
        self.tan_results.get(&id)
    }
    // aliases
    pub fn ingest_tx(&mut self, tx: Tx) -> Result<(), String> { self.accept_tx(tx) }
    pub fn submit_tx(&mut self, tx: Tx) -> Result<(), String> { self.accept_tx(tx) }
    pub fn process_tx(&mut self, tx: Tx) -> Result<(), String> { self.accept_tx(tx) }
    pub fn handle_tx(&mut self, tx: Tx) -> Result<(), String> { self.accept_tx(tx) }
    pub fn intake_tx(&mut self, tx: Tx) -> Result<(), String> { self.accept_tx(tx) }
    pub fn append_tx_with_fee(&mut self, tx: Tx) -> Result<(), String> { self.accept_tx(tx) }
    pub fn accept_transaction(&mut self, tx: Tx) -> Result<(), String> { self.accept_tx(tx) }
    pub fn ingest(&mut self, tx: Tx) -> Result<(), String> { self.accept_tx(tx) }
    pub fn submit(&mut self, tx: Tx) -> Result<(), String> { self.accept_tx(tx) }
    pub fn process(&mut self, tx: Tx) -> Result<(), String> { self.accept_tx(tx) }
    pub fn handle(&mut self, tx: Tx) -> Result<(), String> { self.accept_tx(tx) }
    pub fn intake(&mut self, tx: Tx) -> Result<(), String> { self.accept_tx(tx) }
}
