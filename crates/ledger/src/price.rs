use set_core::PriceEntry;
use std::path::Path;

pub struct PriceLedger {
    pub log: crate::DualLog<PriceEntry>,
    pub entries: Vec<PriceEntry>,
}

impl PriceLedger {
    pub fn new(dir: &Path, name: &str) -> Self {
        let log = crate::DualLog::new(dir, name);
        let entries: Vec<PriceEntry> = log.read_all().unwrap_or_default();
        Self { log, entries }
    }
    pub fn append(&mut self, e: PriceEntry) -> Result<(), String> {
        let mut e = e;
        e.location = e.price;
        self.log.append(&e).map_err(|e| e.to_string())?;
        self.entries.push(e);
        Ok(())
    }
    pub fn by_token(&self, token: u64) -> Vec<PriceEntry> {
        self.entries.iter().filter(|e| e.token == token).cloned().collect()
    }
}
