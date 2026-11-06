use crc32fast::hash;
use serde::{Deserialize, Serialize};
use set_core::encoding::{decode, encode};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub struct DualLog<T> {
    a: PathBuf,
    b: PathBuf,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> DualLog<T>
where
    T: Serialize + for<'de> Deserialize<'de> + PartialEq,
{
    pub fn new(dir: &Path, name: &str) -> Self {
        let a = dir.join(format!("{}.a", name));
        let b = dir.join(format!("{}.b", name));
        Self {
            a,
            b,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn append(&self, item: &T) -> std::io::Result<()> {
        let bytes = encode(item).map_err(|e| std::io::Error::other(e.to_string()))?;
        let crc = hash(&bytes);
        // write to both
        for path in [&self.a, &self.b] {
            let mut f = OpenOptions::new().create(true).append(true).open(path)?;
            f.write_all(&(bytes.len() as u32).to_le_bytes())?;
            f.write_all(&crc.to_le_bytes())?;
            f.write_all(&bytes)?;
            f.flush()?;
        }
        Ok(())
    }

    pub fn read_all(&self) -> std::io::Result<Vec<T>> {
        let va = read_file(&self.a)?;
        let vb = read_file(&self.b)?;
        if va != vb {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "ledger copies mismatch",
            ));
        }
        Ok(va)
    }

    pub fn rebuild_from(&self, items: &[T]) -> std::io::Result<()> {
        // truncate and rewrite both
        for path in [&self.a, &self.b] {
            let mut f = File::create(path)?;
            for item in items {
                let bytes = encode(item).map_err(|e| std::io::Error::other(e.to_string()))?;
                let crc = hash(&bytes);
                f.write_all(&(bytes.len() as u32).to_le_bytes())?;
                f.write_all(&crc.to_le_bytes())?;
                f.write_all(&bytes)?;
            }
            f.flush()?;
        }
        Ok(())
    }
}

fn read_file<T>(path: &Path) -> std::io::Result<Vec<T>>
where
    T: for<'de> Deserialize<'de> + PartialEq,
{
    if !path.exists() {
        return Ok(vec![]);
    }
    let mut f = File::open(path)?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    let mut out = Vec::new();
    let mut i = 0;
    while i < buf.len() {
        if i + 8 > buf.len() {
            break;
        }
        let len = u32::from_le_bytes(buf[i..i + 4].try_into().unwrap()) as usize;
        let crc = u32::from_le_bytes(buf[i + 4..i + 8].try_into().unwrap());
        i += 8;
        if i + len > buf.len() {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "truncated record"));
        }
        let bytes = &buf[i..i + len];
        if hash(bytes) != crc {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "crc mismatch"));
        }
        let item: T = decode(bytes).map_err(|e| std::io::Error::other(e.to_string()))?;
        out.push(item);
        i += len;
    }
    Ok(out)
}
