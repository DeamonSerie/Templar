use secp256k1::{Message, PublicKey, Secp256k1, SecretKey, SECP256K1};
use sha2::{Digest, Sha256};

pub struct Keypair {
    sk: SecretKey,
    pk: PublicKey,
}

impl Keypair {
    pub fn generate() -> Self {
        let secp = Secp256k1::new();
        let mut rng = rand::rng();
        let (sk, pk) = secp.generate_keypair(&mut rng);
        Self { sk, pk }
    }
    pub fn public_key(&self) -> Vec<u8> {
        self.pk.serialize().to_vec()
    }
    pub fn sign(&self, data: &[u8]) -> Vec<u8> {
        let h = sha256(data);
        let msg = Message::from_digest(h);
        SECP256K1.sign_ecdsa(msg, &self.sk).serialize_compact().to_vec()
    }
}

pub fn verify(pubkey: &[u8], data: &[u8], sig: &[u8]) -> bool {
    let pk = match PublicKey::from_slice(pubkey) {
        Ok(p) => p,
        Err(_) => return false,
    };
    if sig.len() < 64 {
        return false;
    }
    let s = match secp256k1::ecdsa::Signature::from_compact(&sig[..64]) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let h = sha256(data);
    let msg = Message::from_digest(h);
    SECP256K1.verify_ecdsa(msg, &s, &pk).is_ok()
}

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(data);
    h.finalize().into()
}
