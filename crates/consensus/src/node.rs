use set_core::{NodeRole, WalletId};

#[derive(Clone, Debug)]
pub struct ValidatorNode {
    pub id: WalletId,
    pub role: NodeRole,
}

#[derive(Clone, Debug)]
pub struct CheckerNode {
    pub id: WalletId,
    pub role: NodeRole,
}

impl ValidatorNode {
    pub fn new(id: WalletId) -> Self {
        Self { id, role: NodeRole::Validator }
    }
}
impl CheckerNode {
    pub fn new(id: WalletId) -> Self {
        Self { id, role: NodeRole::Checker }
    }
}
