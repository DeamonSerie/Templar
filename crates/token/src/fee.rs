use set_core::{Tx, WalletId, INTEGRAL_TOKEN_ID};
use std::borrow::Borrow;

pub fn fee_payer<T: Borrow<Tx>>(tx: T) -> Option<WalletId> {
    match tx.borrow() {
        Tx::Transfer { from, .. } => Some(from.clone()),
        Tx::Pump { wallet, .. } => Some(wallet.clone()),
        Tx::Dump { wallet, .. } => Some(wallet.clone()),
        _ => None,
    }
}

pub fn fee_amount<T: Borrow<Tx>>(tx: T) -> f64 {
    match tx.borrow() {
        Tx::Transfer { amount, .. }
        | Tx::Pump { amount, .. }
        | Tx::Dump { amount, .. } => 0.01 * amount,
        _ => 0.0,
    }
}

pub fn burn_tx_for<T: Borrow<Tx>>(tx: T) -> Option<Tx> {
    let tx = tx.borrow();
    let payer = fee_payer(tx)?;
    let amt = fee_amount(tx);
    if amt == 0.0 {
        return None;
    }
    Some(Tx::Burn {
        wallet: payer,
        token: INTEGRAL_TOKEN_ID,
        amount: amt,
    })
}
