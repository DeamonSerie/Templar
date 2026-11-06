use set_core::{TokenId, Tx, INTEGRAL_TOKEN_ID};

pub fn recompute_price(token: TokenId, txs: &[Tx], cells: u64) -> u64 {
    if cells == 0 {
        return 0;
    }
    let mut p: f64 = 0.0;
    let integral = token == INTEGRAL_TOKEN_ID;
    for tx in txs {
        match tx {
            Tx::Pump { token: t, amount, .. } if *t == token => {
                if integral {
                    p -= *amount;
                } else {
                    p += *amount;
                }
            }
            Tx::Dump { token: t, amount, .. } if *t == token => {
                if integral {
                    p += *amount;
                } else {
                    p -= *amount;
                }
            }
            Tx::Transfer { token: t, amount, .. } if *t == token => {
                p += *amount;
            }
            Tx::Burn { token: t, amount, .. } if *t == token => {
                p += *amount;
            }
            _ => {}
        }
    }
    let cellsf = cells as f64;
    let v = (p % cellsf + cellsf) % cellsf;
    v as u64
}

pub fn is_price_fault(stored: u64, expected: u64) -> bool {
    stored != expected
}
