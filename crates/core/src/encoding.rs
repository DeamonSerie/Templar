use serde::{Deserialize, Serialize};

pub fn encode<T: Serialize>(v: &T) -> Result<Vec<u8>, bincode::Error> {
    bincode::serialize(v)
}

pub fn decode<T: for<'de> Deserialize<'de>>(b: &[u8]) -> Result<T, bincode::Error> {
    bincode::deserialize(b)
}
