use chrono::Utc;
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::io::Write;

#[derive(Debug, Serialize, Deserialize)]
pub struct PendingTransaction {
    timestamp: String,
    client: PeerId,
    current_bid: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompletedTransaction {
    timestamp: String,
    hash: String,
    client: PeerId,
    provider: PeerId,
    tokens: f64,
    input: String,
    output: String,
}

impl PendingTransaction {
    pub fn new(client: PeerId, current_bid: f64) -> Self {
        let timestamp = Utc::now().to_string();
        Self {
            timestamp,
            client,
            current_bid,
        }
    }

    pub fn complete(self, provider: PeerId, input: String, output: String) -> CompletedTransaction {
        let timestamp = Utc::now().to_string();
        let record = format!(
            "{timestamp}{}{provider}{}{input}{output}",
            self.client, self.current_bid
        );
        let mut hasher = Sha3_256::new();
        let _ = hasher
            .write(record.as_bytes())
            .expect("failed to write to hasher buffer");
        let hash_vec = hasher.finalize();
        let hash = String::from_utf8_lossy(&hash_vec).to_string();
        CompletedTransaction {
            timestamp,
            hash,
            client: self.client,
            tokens: self.current_bid,
            provider,
            input,
            output,
        }
    }
}
