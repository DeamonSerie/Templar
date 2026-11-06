use set_core::{TokenId, WalletId, INTEGRAL_TOKEN_ID};

pub fn is_validator(wallet: &WalletId, validators: &[WalletId]) -> bool {
    validators.iter().any(|v| v == wallet)
}

pub fn can_pump_integral(wallet: &WalletId, validators: &[WalletId]) -> bool {
    is_validator(wallet, validators)
}

pub fn validate_pump(wallet: &WalletId, token: TokenId, validators: &[WalletId]) -> Result<(), String> {
    if token == INTEGRAL_TOKEN_ID && !is_validator(wallet, validators) {
        return Err("integral minting restricted".into());
    }
    Ok(())
}

pub fn reject_integral_pump(wallet: &WalletId, token: TokenId, validators: &[WalletId]) -> Result<(), String> {
    validate_pump(wallet, token, validators)
}

pub fn check_pump_access(wallet: &WalletId, token: TokenId, validators: &[WalletId]) -> Result<(), String> {
    validate_pump(wallet, token, validators)
}

pub fn reject_pump(wallet: &WalletId, token: TokenId, validators: &[WalletId]) -> Result<(), String> {
    validate_pump(wallet, token, validators)
}

pub fn allowed_to_pump(wallet: &WalletId, token: TokenId, validators: &[WalletId]) -> bool {
    validate_pump(wallet, token, validators).is_ok()
}

pub fn can_mint(wallet: &WalletId, token: TokenId, validators: &[WalletId]) -> bool {
    validate_pump(wallet, token, validators).is_ok()
}

pub fn access_allowed(wallet: &WalletId, token: TokenId, validators: &[WalletId]) -> bool {
    validate_pump(wallet, token, validators).is_ok()
}
