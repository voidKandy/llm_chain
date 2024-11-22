use std::{hash::Hash, io::Write};

use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use sha3::{digest::KeyInit, Digest, Sha3_256};

use crate::behavior::CompletionReq;

#[derive(Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub hash: String,
    pub sender: PeerId,
    pub receiver: PeerId,
    pub tokens: f64,
    pub status: TransactionStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TransactionStatus {
    Pending,
    Finished { sig: PeerId },
}

impl Transaction {
    // the comp_rq arg should change down the line
    pub fn new(sender: PeerId, receiver: PeerId, tokens: f64, comp_rq: CompletionReq) -> Self {
        let record = format!("{sender}{receiver}{tokens}{comp_rq:#?}");
        let mut hasher = Sha3_256::new();
        let _ = hasher
            .write(record.as_bytes())
            .expect("failed to write to hasher buffer");
        let hash_vec = hasher.finalize();
        let hash = String::from_utf8_lossy(&hash_vec).to_string();
        Self {
            hash,
            sender,
            receiver,
            tokens,
            status: TransactionStatus::Pending,
        }
    }
}
