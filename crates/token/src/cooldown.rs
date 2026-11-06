use set_core::{TokenId, WalletId};
use std::collections::BTreeMap;

pub const COOLDOWN_TXS: u64 = 20;
pub const LARGE_MOVEMENT_THRESHOLD_PCT: u64 = 10;

#[derive(Default)]
pub struct Cooldown {
    last: BTreeMap<(WalletId, TokenId), u64>,
}

impl Cooldown {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn can_act(&self, wallet: &WalletId, token: TokenId, now: u64) -> bool {
        match self.last.get(&(wallet.clone(), token)) {
            Some(&prev) => now.saturating_sub(prev) >= COOLDOWN_TXS,
            None => true,
        }
    }
    pub fn record(&mut self, wallet: &WalletId, token: TokenId, now: u64) {
        self.last.insert((wallet.clone(), token), now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cooldown_blocks() {
        let mut c = Cooldown::new();
        let w = vec![1u8];
        assert!(c.can_act(&w, 1, 0));
        c.record(&w, 1, 0);
        assert!(!c.can_act(&w, 1, 5));
        assert!(c.can_act(&w, 1, 20));
    }
}
