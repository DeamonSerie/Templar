use set_core::{Wallet, TokenId};

pub fn mint(wallet: &mut Wallet, token: TokenId, amount: f64) {
    *wallet.balances.entry(token).or_insert(0.0) += amount;
}

pub fn transfer(from: &mut Wallet, to: &mut Wallet, token: TokenId, amount: f64) -> Result<(), String> {
    let bal = from.balances.get(&token).copied().unwrap_or(0.0);
    if bal < amount {
        return Err("insufficient balance".into());
    }
    *from.balances.entry(token).or_insert(0.0) -= amount;
    *to.balances.entry(token).or_insert(0.0) += amount;
    Ok(())
}
