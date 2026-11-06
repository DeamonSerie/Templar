pub fn hash_block(b: &crate::types::Block) -> [u8; 32] {
    let bytes = crate::encoding::encode(b).unwrap_or_default();
    crate::crypto::sha256(&bytes)
}
