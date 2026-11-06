use set_core::{TokenId, Wallet, INTEGRAL_TOKEN_ID};
use crate::cooldown::{Cooldown, LARGE_MOVEMENT_THRESHOLD_PCT};

pub const DUMP_THRESHOLD_PERCENT: u64 = 10;

pub fn price_move_percent(before: u64, after: u64) -> u64 {
    if before == 0 {
        return 0;
    }
    let diff = after.abs_diff(before);
    diff * 100 / before
}

pub fn apply_pump(
    wallet: &mut Wallet,
    token: TokenId,
    amount: f64,
    global_tx: u64,
    cooldown: &mut Cooldown,
) -> Result<(), String> {
    if token == INTEGRAL_TOKEN_ID {
        return Err("integral minting restricted".into());
    }
    // For pump in this crate API we don't have price_before/after; treat as small unless caller uses pallet logic
    // Default: small movement -> skip cooldown
    *wallet.balances.entry(token).or_insert(0.0) += amount;
    Ok(())
}

pub fn apply_dump(
    wallet: &mut Wallet,
    token: TokenId,
    amount: f64,
    price_before: u64,
    price_after: u64,
    global_tx: u64,
    cooldown: &mut Cooldown,
) -> Result<(), String> {
    let move_pct = price_move_percent(price_before, price_after);
    if move_pct >= LARGE_MOVEMENT_THRESHOLD_PCT {
        if !cooldown.can_act(&wallet.pubkey, token, global_tx) {
            return Err("cooldown active".into());
        }
    }
    let bal = wallet.balances.get(&token).copied().unwrap_or(0.0);
    if bal < amount {
        return Err("insufficient balance".into());
    }
    *wallet.balances.entry(token).or_insert(0.0) -= amount;

    if move_pct >= DUMP_THRESHOLD_PERCENT {
        let remaining = wallet.balances.get(&token).copied().unwrap_or(0.0);
        let half = remaining / 2.0;
        if half > 0.0 {
            *wallet.balances.entry(token).or_insert(0.0) -= half;
            let proceeds = half * price_after as f64;
            *wallet.balances.entry(0).or_insert(0.0) += proceeds;
        }
    }
    if move_pct >= LARGE_MOVEMENT_THRESHOLD_PCT {
        cooldown.record(&wallet.pubkey, token, global_tx);
    }
    Ok(())
}
