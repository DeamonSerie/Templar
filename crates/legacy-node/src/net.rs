use set_core::{Block, Tx};
use set_consensus::Vote;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum NetMsg {
    Tx(Tx),
    Block(Block),
    PriceFixProposal {
        id: u64,
        token: u64,
        index: u64,
        correct_price: u64,
        original_index: u64,
    },
    Vote(Vote),
    LedgerSyncRequest,
    LedgerSyncResponse {
        txs: Vec<u8>,
        prices: Vec<u8>,
    },
    TanQuery {
        id: u64,
        raw: Vec<u8>,
    },
    TanResult {
        id: u64,
        result: Vec<u8>,
    },
}

pub fn send_msg(stream: &mut std::net::TcpStream, msg: &NetMsg) -> std::io::Result<()> {
    let bytes = bincode::serialize(msg).map_err(|e| std::io::Error::other(e.to_string()))?;
    let len = bytes.len() as u32;
    use std::io::Write;
    stream.write_all(&len.to_le_bytes())?;
    stream.write_all(&bytes)?;
    stream.flush()
}

pub fn recv_msg(stream: &mut std::net::TcpStream) -> std::io::Result<NetMsg> {
    use std::io::Read;
    let mut lenbuf = [0u8; 4];
    stream.read_exact(&mut lenbuf)?;
    let len = u32::from_le_bytes(lenbuf) as usize;
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf)?;
    bincode::deserialize(&buf).map_err(|e| std::io::Error::other(e.to_string()))
}

pub fn serve(addr: &str, handler: impl Fn(NetMsg) -> Option<NetMsg> + Send + Sync + 'static) {
    let listener = match std::net::TcpListener::bind(addr) {
        Ok(l) => l,
        Err(_) => return,
    };
    let handler = std::sync::Arc::new(handler);
    for mut s in listener.incoming().flatten() {
        let h = handler.clone();
        std::thread::spawn(move || {
            while let Ok(msg) = recv_msg(&mut s) {
                if let Some(resp) = h(msg) {
                    let _ = send_msg(&mut s, &resp);
                }
            }
        });
    }
}

pub fn client_send(addr: &str, msg: NetMsg) -> std::io::Result<Option<NetMsg>> {
    let mut stream = std::net::TcpStream::connect(addr)?;
    send_msg(&mut stream, &msg)?;
    // try read response with short timeout? blocking; attempt
    stream.set_read_timeout(Some(std::time::Duration::from_millis(200))).ok();
    match recv_msg(&mut stream) {
        Ok(m) => Ok(Some(m)),
        Err(_) => Ok(None),
    }
}
