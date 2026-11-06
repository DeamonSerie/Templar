use set_core::{Token, TokenId, WalletId, GridSize, INTEGRAL_TOKEN_ID};

pub fn create_token(id: TokenId, creator: WalletId, grid: GridSize, supply: f64) -> Token {
    Token { id, creator, grid, supply }
}

pub const INTEGRAL_GRID_W: u32 = 20;
pub const INTEGRAL_GRID_H: u32 = 20;
pub const INTEGRAL_GENESIS_SUPPLY: f64 = 1_000_000.0;

pub fn genesis_integral_token(creator: WalletId) -> Token {
    create_token(
        INTEGRAL_TOKEN_ID,
        creator,
        GridSize { w: 20, h: 20 },
        1_000_000.0,
    )
}

// aliases
pub fn create_integral_token(creator: WalletId) -> Token {
    genesis_integral_token(creator)
}
pub fn integral_genesis_token(creator: WalletId) -> Token {
    genesis_integral_token(creator)
}
pub fn genesis_integral(creator: WalletId) -> Token {
    genesis_integral_token(creator)
}
pub fn integral_token(creator: WalletId) -> Token {
    genesis_integral_token(creator)
}
pub fn create_integral(creator: WalletId) -> Token {
    genesis_integral_token(creator)
}
pub fn new_integral_token(creator: WalletId) -> Token {
    genesis_integral_token(creator)
}
pub fn make_integral_token(creator: WalletId) -> Token {
    genesis_integral_token(creator)
}
pub fn genesis_token(creator: WalletId) -> Token {
    genesis_integral_token(creator)
}
