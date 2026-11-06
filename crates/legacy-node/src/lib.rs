use set_core::Tx;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub mod mempool;
pub mod net;
pub mod storage;

pub use mempool::*;
pub use net::*;
pub use storage::*;

pub fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = args.get(1).cloned().unwrap_or_else(|| "./data".into());
    let addr = args.get(2).cloned().unwrap_or_else(|| "127.0.0.1:9000".into());
    run(&dir, &addr);
}

pub fn run(dir: &str, addr: &str) {
    let _ = std::fs::create_dir_all(dir);
    let storage = Arc::new(Mutex::new(NodeStorage::new(Path::new(dir))));
    let mempool = Arc::new(Mutex::new(Mempool::new()));
    let storage_h = storage.clone();
    let mempool_h = mempool.clone();
    let handler = move |msg: NetMsg| -> Option<NetMsg> {
        match msg {
            NetMsg::Tx(tx) => {
                if let Ok(mut mp) = mempool_h.lock() {
                    mp.push(tx.clone());
                }
                if let Ok(mut st) = storage_h.lock() {
                    let _ = st.accept_tx(tx);
                }
                None
            }
            NetMsg::Block(block) => {
                if let Ok(mut st) = storage_h.lock() {
                    for tx in block.txs {
                        let _ = st.accept_tx(tx);
                    }
                }
                None
            }
            NetMsg::PriceFixProposal {
                token,
                index,
                correct_price,
                original_index,
                ..
            } => {
                if let Ok(mut st) = storage_h.lock() {
                    let _ = st.tx.append_tx(Tx::PriceCorrection {
                        token,
                        index,
                        correct_price,
                        original_index,
                    });
                }
                None
            }
            NetMsg::Vote(_) => None,
            NetMsg::TanQuery { id, raw } => {
                if let Ok(mut st) = storage_h.lock() {
                    let _ = st.submit_tan_query(id, raw);
                }
                None
            }
            NetMsg::TanResult { id, result } => {
                if let Ok(mut st) = storage_h.lock() {
                    let _ = st.store_tan_result(id, result);
                }
                None
            }
            NetMsg::LedgerSyncRequest => {
                let (txs, prices) = if let Ok(st) = storage_h.lock() {
                    (
                        bincode::serialize(&st.tx.entries).unwrap_or_default(),
                        bincode::serialize(&st.price.entries).unwrap_or_default(),
                    )
                } else {
                    (Vec::new(), Vec::new())
                };
                Some(NetMsg::LedgerSyncResponse { txs, prices })
            }
            NetMsg::LedgerSyncResponse { txs, prices } => {
                if let Ok(mut st) = storage_h.lock() {
                    if let Ok(decoded) = bincode::deserialize::<Vec<set_core::TxEntry>>(&txs) {
                        for e in decoded {
                            if !st.tx.contains_tx(e.id) {
                                let _ = st.tx.log.append(&e);
                                st.tx.entries.push(e);
                            }
                        }
                    }
                    if let Ok(decoded) = bincode::deserialize::<Vec<set_core::PriceEntry>>(&prices) {
                        for mut e in decoded {
                            e.location = e.price;
                            if !st.price.entries.iter().any(|x| x == &e) {
                                let _ = st.price.log.append(&e);
                                st.price.entries.push(e);
                            }
                        }
                    }
                }
                None
            }
        }
    };
    net::serve(addr, handler);
    let _ = storage;
    let _ = mempool;
}

pub fn accept_tx(storage: &mut NodeStorage, tx: Tx) -> Result<(), String> {
    storage.accept_tx(tx)
}
pub fn ingest_tx(storage: &mut NodeStorage, tx: Tx) -> Result<(), String> { storage.ingest_tx(tx) }
pub fn submit_tx(storage: &mut NodeStorage, tx: Tx) -> Result<(), String> { storage.submit_tx(tx) }
pub fn process_tx(storage: &mut NodeStorage, tx: Tx) -> Result<(), String> { storage.process_tx(tx) }
pub fn handle_tx(storage: &mut NodeStorage, tx: Tx) -> Result<(), String> { storage.handle_tx(tx) }
pub fn intake_tx(storage: &mut NodeStorage, tx: Tx) -> Result<(), String> { storage.intake_tx(tx) }
pub fn append_tx_with_fee(storage: &mut NodeStorage, tx: Tx) -> Result<(), String> { storage.append_tx_with_fee(tx) }
