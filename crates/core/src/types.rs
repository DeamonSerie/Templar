use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type TokenId = u64;
pub type TxId = u64;
pub type Height = u64;
pub type WalletId = Vec<u8>;

pub const INTEGRAL_TOKEN_ID: TokenId = u64::MAX;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct GridSize {
    pub w: u32,
    pub h: u32,
}

impl GridSize {
    pub fn cells(&self) -> u64 {
        self.w as u64 * self.h as u64
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Token {
    pub id: TokenId,
    pub creator: WalletId,
    pub grid: GridSize,
    pub supply: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Wallet {
    pub pubkey: WalletId,
    pub balances: BTreeMap<TokenId, f64>,
}

impl Wallet {
    pub fn new(pubkey: WalletId) -> Self {
        Self {
            pubkey,
            balances: BTreeMap::new(),
        }
    }
    pub fn balance(&self, token: TokenId) -> f64 {
        self.balances.get(&token).copied().unwrap_or(0.0)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Tx {
    Transfer {
        from: WalletId,
        to: WalletId,
        token: TokenId,
        amount: f64,
    },
    CreateToken {
        creator: WalletId,
        grid: GridSize,
        supply: f64,
    },
    Pump {
        wallet: WalletId,
        token: TokenId,
        amount: f64,
    },
    Dump {
        wallet: WalletId,
        token: TokenId,
        amount: f64,
    },
    Burn {
        wallet: WalletId,
        token: TokenId,
        amount: f64,
    },
    ContractCall {
        caller: WalletId,
        contract: Vec<u8>,
        calldata: Vec<u8>,
    },
    PriceCorrection {
        token: TokenId,
        index: u64,
        correct_price: u64,
        original_index: u64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Block {
    pub height: Height,
    pub prev_hash: [u8; 32],
    pub txs: Vec<Tx>,
    pub proposer: WalletId,
    pub sig: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PriceEntry {
    pub token: TokenId,
    pub location: u64,
    pub price: u64,
    pub tx_backing: TxId,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TxEntry {
    pub id: TxId,
    pub tx: Tx,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum NodeRole {
    Validator,
    Checker,
}
