use set_core::Tx;

#[derive(Default)]
pub struct Mempool {
    pub txs: Vec<Tx>,
}

impl Mempool {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn push(&mut self, tx: Tx) {
        self.txs.push(tx);
    }
    pub fn drain(&mut self) -> Vec<Tx> {
        std::mem::take(&mut self.txs)
    }
}
