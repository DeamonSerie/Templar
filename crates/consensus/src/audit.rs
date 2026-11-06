use set_core::{TokenId, Tx};
use set_ledger::{PriceLedger, TxLedger};
use set_token::recompute_price;

#[derive(Clone, Debug)]
pub struct PriceFault {
    pub token: TokenId,
    pub location: u64,
    pub stored: u64,
    pub expected: u64,
    pub tx_backing: u64,
}

pub fn audit_prices<F>(price_ledger: &PriceLedger, tx_ledger: &TxLedger, cells_for: F) -> Vec<PriceFault>
where
    F: Fn(TokenId) -> u64,
{
    let mut faults = Vec::new();
    for entry in &price_ledger.entries {
        let txs: Vec<Tx> = tx_ledger
            .entries
            .iter()
            .filter(|te| te.id <= entry.tx_backing)
            .map(|te| te.tx.clone())
            .collect();
    let cells = cells_for(entry.token);
    let expected = recompute_price(entry.token, &txs, cells);
    if entry.price != expected {
        faults.push(PriceFault {
            token: entry.token,
            location: entry.location,
            stored: entry.price,
            expected,
            tx_backing: entry.tx_backing,
        });
    }
    if entry.location != entry.price {
        faults.push(PriceFault {
            token: entry.token,
            location: entry.location,
            stored: entry.location,
            expected: entry.price,
            tx_backing: entry.tx_backing,
        });
    }
    }
    faults
}

pub fn distribution_amount(total_extracted: f64) -> f64 {
    total_extracted / 100_000.0
}
